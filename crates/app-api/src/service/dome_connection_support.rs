use crate::service::*;
use kukuri_core::SpatialContextV1;
use serde::{Deserialize, Serialize};

pub(crate) const PROPOSAL_PREFIX: &str = "metaverse/dome-connections/proposals";
pub(crate) const SELECTION_PREFIX: &str = "metaverse/dome-connections/selections";
pub(crate) const CONNECTION_PREFIX: &str = "metaverse/dome-connections/agreements";
pub(crate) const LOCAL_PROPOSAL_RATE_WINDOW_MS: i64 = 10 * 60 * 1_000;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct DomeProposalStateDocV1 {
    pub(crate) proposal: DomeConnectionProposalV1,
    pub(crate) connection_id: String,
    pub(crate) proposal_envelope_id: EnvelopeId,
    pub(crate) proposer_agreement_envelope_id: EnvelopeId,
    pub(crate) terminal_reason: Option<DomeConnectionTerminalReasonV1>,
    pub(crate) terminal_event_envelope_id: Option<EnvelopeId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct DomeSelectionStateDocV1 {
    pub(crate) selection: DomeProposalSelectionV1,
    pub(crate) envelope_id: EnvelopeId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct DomeConnectionStateDocV1 {
    pub(crate) record: DomeConnectionRecordV1,
    pub(crate) proposer_agreement_envelope_id: EnvelopeId,
    pub(crate) receiver_agreement_envelope_id: EnvelopeId,
    pub(crate) lifecycle_envelope_id: EnvelopeId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct DomeProposalTerminalEventV1 {
    pub(crate) proposal_id: String,
    pub(crate) spatial_context: SpatialContextV1,
    pub(crate) actor_pubkey: Pubkey,
    pub(crate) reason: DomeConnectionTerminalReasonV1,
    pub(crate) updated_at: i64,
}

impl AppService {
    pub(crate) async fn terminate_dome_connection_with_reason(
        &self,
        spatial_context: &SpatialContextV1,
        connection_id: &str,
        reason: DomeConnectionTerminalReasonV1,
    ) -> Result<DomeConnectionView> {
        let replica = self
            .dome_connection_context_replica(spatial_context)
            .await?;
        let mut state = self
            .fetch_dome_connection_state(&replica, connection_id)
            .await?
            .context("Dome Connection was not found")?;
        let actor = Pubkey::from(self.current_author_pubkey());
        if actor != state.record.agreement.proposer.owner_pubkey
            && actor != state.record.agreement.receiver.owner_pubkey
        {
            anyhow::bail!("only an endpoint owner can terminate a Dome Connection");
        }
        if state.record.status != DomeConnectionStatusV1::Revoked {
            state.record.status = DomeConnectionStatusV1::Draining;
            state.record.lifecycle_generation += 1;
            state.record.lifecycle_actor = Some(actor.clone());
            state.record.lifecycle_reason = Some(reason);
            self.persist_connection_lifecycle(&replica, &mut state)
                .await?;
            state.record.status = DomeConnectionStatusV1::Revoked;
            state.record.lifecycle_generation += 1;
            self.persist_connection_lifecycle(&replica, &mut state)
                .await?;
        }
        self.publish_dome_topology_hint(
            &state.record.agreement.spatial_context,
            &state.record.agreement.connection_id,
        )
        .await?;
        Ok(DomeConnectionView {
            record: state.record,
        })
    }

    pub(crate) async fn reconcile_blocked_dome_connections(
        &self,
        target_pubkey: &Pubkey,
    ) -> Result<()> {
        let mut contexts = self
            .subscription_registry
            .subscriptions
            .lock()
            .await
            .keys()
            .map(|topic_id| SpatialContextV1::Topic {
                topic_id: TopicId::new(topic_id.clone()),
            })
            .collect::<Vec<_>>();
        contexts.extend(
            self.joined_private_channels
                .lock()
                .await
                .values()
                .map(|state| SpatialContextV1::Channel {
                    topic_id: TopicId::new(state.topic_id.clone()),
                    channel_id: state.channel_id.clone(),
                }),
        );
        contexts.sort_by_key(SpatialContextV1::canonical_id);
        contexts.dedup_by(|left, right| left.canonical_id() == right.canonical_id());

        let local_owner = Pubkey::from(self.current_author_pubkey());
        let mut revoked_connection_ids = Vec::new();
        for context in contexts {
            let Ok(topology) = self.list_dome_connection_topology(context.clone()).await else {
                continue;
            };
            for connection in topology.connections {
                let agreement = &connection.record.agreement;
                let owners_match = (agreement.proposer.owner_pubkey == local_owner
                    && agreement.receiver.owner_pubkey == *target_pubkey)
                    || (agreement.receiver.owner_pubkey == local_owner
                        && agreement.proposer.owner_pubkey == *target_pubkey);
                if owners_match && connection.record.status != DomeConnectionStatusV1::Revoked {
                    let revoked = self
                        .terminate_dome_connection_with_reason(
                            &context,
                            agreement.connection_id.as_str(),
                            DomeConnectionTerminalReasonV1::OwnersBlocked,
                        )
                        .await?;
                    revoked_connection_ids.push(revoked.record.agreement.connection_id);
                }
            }
        }

        let mut sessions = self.dome_host_sessions.lock().await;
        for runtime in sessions.values_mut() {
            if runtime.lease().owner_pubkey == local_owner {
                runtime.evict_participant(target_pubkey);
            }
            for connection_id in &revoked_connection_ids {
                runtime.revoke_transition_access(target_pubkey, Some(connection_id.as_str()));
            }
        }
        Ok(())
    }
}
