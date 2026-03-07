use anyhow::Result;
use sqlx::{PgConnection, Row};

pub const DEFAULT_TOPIC_SERVICES: [(&str, &str); 2] =
    [("bootstrap", "public"), ("relay", "public")];

pub async fn default_topic_services_are_active(
    conn: &mut PgConnection,
    topic_id: &str,
) -> Result<bool> {
    let row = sqlx::query(
        "SELECT COUNT(*) AS active_count
         FROM cn_admin.topic_services
         WHERE topic_id = $1
           AND is_active = TRUE
           AND ((role = 'bootstrap' AND scope = 'public')
             OR (role = 'relay' AND scope = 'public'))",
    )
    .bind(topic_id)
    .fetch_one(&mut *conn)
    .await?;

    let active_count: i64 = row.try_get("active_count")?;
    Ok(active_count == DEFAULT_TOPIC_SERVICES.len() as i64)
}

pub async fn sync_default_topic_services(
    conn: &mut PgConnection,
    topic_id: &str,
    active: bool,
    updated_by: &str,
) -> Result<()> {
    if active {
        for (role, scope) in DEFAULT_TOPIC_SERVICES {
            sqlx::query(
                "INSERT INTO cn_admin.topic_services
                     (topic_id, role, scope, is_active, updated_by)
                 VALUES ($1, $2, $3, TRUE, $4)
                 ON CONFLICT (topic_id, role, scope) DO UPDATE
                     SET is_active = TRUE,
                         updated_at = NOW(),
                         updated_by = EXCLUDED.updated_by",
            )
            .bind(topic_id)
            .bind(role)
            .bind(scope)
            .bind(updated_by)
            .execute(&mut *conn)
            .await?;
        }
        return Ok(());
    }

    for (role, scope) in DEFAULT_TOPIC_SERVICES {
        sqlx::query(
            "UPDATE cn_admin.topic_services
             SET is_active = FALSE,
                 updated_at = NOW(),
                 updated_by = $4
             WHERE topic_id = $1
               AND role = $2
               AND scope = $3
               AND is_active = TRUE",
        )
        .bind(topic_id)
        .bind(role)
        .bind(scope)
        .bind(updated_by)
        .execute(&mut *conn)
        .await?;
    }

    Ok(())
}
