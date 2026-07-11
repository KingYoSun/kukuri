use anyhow::Result;
use sqlx::PgPool;

use kukuri_cn_core::{get_community_node_report, list_community_node_reports};

use crate::ReportsAction;

pub(super) async fn run(pool: &PgPool, action: ReportsAction) -> Result<()> {
    match action {
        ReportsAction::List { limit, offset } => {
            let reports = list_community_node_reports(pool, limit, offset).await?;
            if reports.is_empty() {
                println!("no reports");
            } else {
                println!(
                    "{} report(s) (limit={} offset={}):",
                    reports.len(),
                    limit,
                    offset
                );
                for report in reports {
                    println!(
                        "{}  {}  {}/{}  capability={}  reason={}  status={}",
                        report.created_at.to_rfc3339(),
                        report.id,
                        report.subject_kind,
                        report.subject_id,
                        report.capability,
                        report.reason,
                        report.status,
                    );
                }
            }
        }
        ReportsAction::Show { id } => match get_community_node_report(pool, id.as_str()).await? {
            Some(report) => {
                println!("id:               {}", report.id);
                println!("created_at:       {}", report.created_at.to_rfc3339());
                println!("status:           {}", report.status);
                println!("subject_kind:     {}", report.subject_kind);
                println!("subject_id:       {}", report.subject_id);
                println!("capability:       {}", report.capability);
                println!("reason:           {}", report.reason);
                println!(
                    "details:          {}",
                    report.details.as_deref().unwrap_or("-")
                );
                println!(
                    "reporter_contact: {}",
                    report.reporter_contact.as_deref().unwrap_or("-")
                );
            }
            None => println!("report not found: {id}"),
        },
    }
    Ok(())
}
