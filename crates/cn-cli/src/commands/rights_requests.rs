use anyhow::Result;
use kukuri_cn_core::{
    action_rights_request, get_rights_request, initialize_database, list_rights_requests,
    transition_rights_request,
};
use kukuri_cn_protocol::RightsRequestStatus;
use sqlx::PgPool;

use crate::{RightsRequestStatusArg, RightsRequestsAction};

pub(super) async fn run(pool: &PgPool, action: RightsRequestsAction) -> Result<()> {
    initialize_database(pool).await?;
    match action {
        RightsRequestsAction::List { limit, offset } => {
            let requests = list_rights_requests(pool, limit, offset).await?;
            if requests.is_empty() {
                println!("no rights requests");
            }
            for request in requests {
                println!(
                    "{}  {}  {}/{}  scope={:?}  status={:?}  version={}",
                    request.created_at.to_rfc3339(),
                    request.id,
                    request.subject_kind,
                    request.subject_id,
                    request.scope_status,
                    request.status,
                    request.version,
                );
            }
        }
        RightsRequestsAction::Show { id } => match get_rights_request(pool, &id).await? {
            Some(request) => println!("{}", serde_json::to_string_pretty(&request)?),
            None => println!("rights request not found: {id}"),
        },
        RightsRequestsAction::Transition {
            id,
            expected_version,
            actor,
            status,
            public_message,
            delivery_status,
        } => {
            let request = transition_rights_request(
                pool,
                &id,
                expected_version,
                &actor,
                status.into(),
                public_message.as_deref(),
                &delivery_status,
            )
            .await?;
            println!(
                "rights request transitioned: id={} status={:?} version={}",
                request.id, request.status, request.version
            );
        }
        RightsRequestsAction::Action {
            id,
            expected_version,
            actor,
            capabilities,
            public_message,
        } => {
            let result = action_rights_request(
                pool,
                &id,
                expected_version,
                &actor,
                capabilities.into_iter().map(Into::into).collect(),
                &public_message,
            )
            .await?;
            println!(
                "rights request actioned: id={} prevention={} removed_index_scopes={} version={}",
                result.request.id,
                result.prevention.decision.id,
                result.prevention.removed_index_scopes.len(),
                result.request.version,
            );
        }
    }
    Ok(())
}

impl From<RightsRequestStatusArg> for RightsRequestStatus {
    fn from(value: RightsRequestStatusArg) -> Self {
        match value {
            RightsRequestStatusArg::Received => Self::Received,
            RightsRequestStatusArg::NeedsInformation => Self::NeedsInformation,
            RightsRequestStatusArg::Reviewing => Self::Reviewing,
            RightsRequestStatusArg::SenderContacting => Self::SenderContacting,
            RightsRequestStatusArg::Actioned => Self::Actioned,
            RightsRequestStatusArg::Declined => Self::Declined,
            RightsRequestStatusArg::OutOfScope => Self::OutOfScope,
            RightsRequestStatusArg::Withdrawn => Self::Withdrawn,
        }
    }
}
