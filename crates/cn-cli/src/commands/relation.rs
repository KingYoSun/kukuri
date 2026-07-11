use anyhow::{Context, Result};
use sqlx::PgPool;

use kukuri_cn_core::PgCoParticipationSource;
use kukuri_cn_indexer::{ArcadeDbConfig, ArcadeDbRelationGraph, analyze_relations};

use crate::RelationAction;

pub(super) async fn run(pool: &PgPool, action: RelationAction) -> Result<()> {
    match action {
        RelationAction::Analyze { limit } => {
            let source = PgCoParticipationSource::new(pool.clone());
            let graph = ArcadeDbRelationGraph::new(ArcadeDbConfig::from_env())
                .context("failed to build ArcadeDB relation graph client")?;
            graph
                .ensure_schema()
                .await
                .context("failed to ensure relation graph schema")?;
            let report = analyze_relations(&source, &graph, limit).await?;
            println!(
                "relation analysis done: {} edge(s) upserted, {} cluster(s) assigned",
                report.edges_upserted, report.clusters_assigned
            );
        }
    }
    Ok(())
}
