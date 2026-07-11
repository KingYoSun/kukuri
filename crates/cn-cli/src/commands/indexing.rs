use anyhow::Result;
use sqlx::PgPool;

use kukuri_cn_core::{
    IndexScopeKind, IndexingRequestStatus, add_supported_topic, approve_indexing_request,
    initialize_database, list_indexing_requests, list_supported_topics, reject_indexing_request,
    remove_supported_topic,
};

use crate::{IndexingRequestAction, SupportedTopicAction};

pub(super) async fn run_supported_topic(pool: &PgPool, action: SupportedTopicAction) -> Result<()> {
    initialize_database(pool).await?;
    match action {
        SupportedTopicAction::Add { kind, id } => {
            let kind = IndexScopeKind::from(kind);
            let entry = add_supported_topic(pool, kind, id.as_str()).await?;
            println!(
                "supported topic added: {} {}",
                entry.kind.as_str(),
                entry.id
            );
        }
        SupportedTopicAction::Remove { kind, id } => {
            let kind = IndexScopeKind::from(kind);
            if remove_supported_topic(pool, kind, id.as_str()).await? {
                println!("supported topic removed: {} {}", kind.as_str(), id);
            } else {
                println!("supported topic not found: {} {}", kind.as_str(), id);
            }
        }
        SupportedTopicAction::List => {
            let entries = list_supported_topics(pool).await?;
            if entries.is_empty() {
                println!("no supported topics");
            } else {
                println!("{} supported topic(s):", entries.len());
                for entry in entries {
                    println!(
                        "{}  {}  {}",
                        entry.created_at.to_rfc3339(),
                        entry.kind.as_str(),
                        entry.id,
                    );
                }
            }
        }
    }
    Ok(())
}

pub(super) async fn run_indexing_request(
    pool: &PgPool,
    action: IndexingRequestAction,
) -> Result<()> {
    initialize_database(pool).await?;
    match action {
        IndexingRequestAction::List {
            status,
            limit,
            offset,
        } => {
            let status = status.map(IndexingRequestStatus::from);
            let requests = list_indexing_requests(pool, status, limit, offset).await?;
            if requests.is_empty() {
                println!("no indexing requests");
            } else {
                println!("{} indexing request(s):", requests.len());
                for request in requests {
                    println!(
                        "{}  {}  {}/{}  requester={}  status={}",
                        request.created_at.to_rfc3339(),
                        request.id,
                        request.kind.as_str(),
                        request.target_id,
                        request.requester_pubkey,
                        request.status.as_str(),
                    );
                }
            }
        }
        IndexingRequestAction::Approve { id } => {
            match approve_indexing_request(pool, id.as_str()).await? {
                Some(request) => println!(
                    "indexing request approved: {} ({} {} now supported)",
                    request.id,
                    request.kind.as_str(),
                    request.target_id,
                ),
                None => println!("indexing request not found: {id}"),
            }
        }
        IndexingRequestAction::Reject { id } => {
            match reject_indexing_request(pool, id.as_str()).await? {
                Some(request) => println!("indexing request rejected: {}", request.id),
                None => println!("indexing request not found: {id}"),
            }
        }
    }
    Ok(())
}
