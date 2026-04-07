use anyhow::Result;

use crate::{
    DocOp, DocQuery, DocsSync, IrohDocsNode, IrohDocsSync, author_replica_id, device_replica_id,
    topic_replica_id,
};

#[tokio::test]
async fn docs_topic_index_roundtrip() -> Result<()> {
    let node = IrohDocsNode::memory().await?;
    let docs = IrohDocsSync::new(node.clone());
    let replica = topic_replica_id("kukuri:topic:docs");

    docs.open_replica(&replica).await?;
    docs.apply_doc_op(
        &replica,
        DocOp::SetJson {
            key: crate::stable_key("timeline", "0001-event"),
            value: serde_json::json!({
                "object_id": "event-1",
                "topic_id": "kukuri:topic:docs"
            }),
        },
    )
    .await?;

    let rows = docs
        .query_replica(&replica, DocQuery::Prefix("timeline/".into()))
        .await?;

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].key, "timeline/0001-event");
    assert!(String::from_utf8(rows[0].value.clone())?.contains("event-1"));

    docs.shutdown().await;
    node.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn private_cursor_not_in_public_replica() -> Result<()> {
    let node = IrohDocsNode::memory().await?;
    let docs = IrohDocsSync::new(node.clone());
    let topic_replica = topic_replica_id("kukuri:topic:docs");
    let author_replica = author_replica_id("f".repeat(64).as_str());
    let device_replica = device_replica_id("f".repeat(64).as_str(), "device-a");

    docs.open_replica(&topic_replica).await?;
    docs.open_replica(&author_replica).await?;
    docs.open_replica(&device_replica).await?;

    docs.apply_doc_op(
        &device_replica,
        DocOp::SetJson {
            key: "cursor/topic/kukuri:topic:docs".into(),
            value: serde_json::json!({ "created_at": 1 }),
        },
    )
    .await?;

    let topic_rows = docs
        .query_replica(&topic_replica, DocQuery::Prefix("cursor/".into()))
        .await?;
    let author_rows = docs
        .query_replica(&author_replica, DocQuery::Prefix("cursor/".into()))
        .await?;
    let device_rows = docs
        .query_replica(&device_replica, DocQuery::Prefix("cursor/".into()))
        .await?;

    assert!(topic_rows.is_empty());
    assert!(author_rows.is_empty());
    assert_eq!(device_rows.len(), 1);
    assert_eq!(device_rows[0].key, "cursor/topic/kukuri:topic:docs");

    docs.shutdown().await;
    node.shutdown().await?;
    Ok(())
}
