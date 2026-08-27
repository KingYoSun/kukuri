use crate::service::*;
use crate::views::{
    ActivateCommunityNodeDomeHostingInput, CloseDomeHostingInput, DomeHostingView,
    PrepareCommunityNodeDomeHostingInput, StartOwnerDomeHostingInput, SubmitDomeSessionInput,
};
use kukuri_core::{
    DOME_HOSTING_MAX_LEASE_MILLIS, DomeHostTargetV1, DomeHostingLeaseV1, DomeHostingRecordV1,
    DomeInstanceStatusV1, SignedDomeHostingAcceptanceV1, SignedDomeHostingLeaseV1,
    SignedDomePhysicsSnapshotV1, SpatialContextV1, accept_dome_hosting_lease,
    activate_dome_hosting_lease, build_signed_dome_hosting_lease, build_signed_dome_session_input,
    close_dome_hosting_lease, resolve_dome_hosting_state,
};

const HOSTING_RECORD_PREFIX: &str = "metaverse/dome-hosting";
const DEFAULT_LEASE_MILLIS: i64 = 24 * 60 * 60 * 1_000;

impl AppService {
    pub async fn get_dome_hosting(
        &self,
        spatial_context: SpatialContextV1,
        instance_id: &str,
    ) -> Result<DomeHostingView> {
        let replica = self.hosting_context_replica(&spatial_context).await?;
        let instance = self
            .hosting_instance(&replica, instance_id)
            .await?
            .context("Dome instance was not found")?;
        let records = self
            .list_dome_hosting_records(&replica, instance_id)
            .await?;
        self.hosting_view(&instance, &records, Utc::now().timestamp_millis())
            .await
    }

    pub async fn start_owner_dome_hosting(
        &self,
        input: StartOwnerDomeHostingInput,
    ) -> Result<DomeHostingView> {
        if input.endpoint_id.trim().is_empty() {
            anyhow::bail!("owner device endpoint id is required");
        }
        let replica = self.hosting_context_replica(&input.spatial_context).await?;
        let instance = self
            .hosting_instance(&replica, &input.instance_id)
            .await?
            .context("Dome instance was not found")?;
        self.ensure_dome_hosting_owner(&instance)?;
        let preset = self
            .fetch_dome_preset_manifest(&instance.preset_ref)
            .await?
            .context("Dome preset manifest is unavailable")?;
        let records = self
            .list_dome_hosting_records(&replica, &input.instance_id)
            .await?;
        let now = Utc::now().timestamp_millis();
        let epoch = next_hosting_epoch(&records);
        let lease = build_signed_dome_hosting_lease(
            self.services.keys.as_ref(),
            DomeHostingLeaseV1 {
                lease_id: format!("lease-{}-{epoch}", instance.instance_id),
                spatial_context: instance.spatial_context.clone(),
                instance_id: instance.instance_id.clone(),
                instance_generation: instance.generation,
                owner_pubkey: instance.owner_pubkey.clone(),
                host: DomeHostTargetV1::OwnerDevice {
                    endpoint_id: input.endpoint_id,
                    host_pubkey: self.services.keys.public_key(),
                },
                manifest_blob_hash: instance.preset_ref.manifest_blob_hash.clone(),
                manifest_version: METAVERSE_WORLD_VERSION,
                epoch,
                issued_at: now,
                expires_at: lease_expiry(now, input.lease_duration_millis)?,
            },
        )?;
        let session_id = format!("dome-session-{}-{epoch}-{now}", instance.instance_id);
        let acceptance =
            accept_dome_hosting_lease(self.services.keys.as_ref(), &lease, &session_id, now)?;
        let activation =
            activate_dome_hosting_lease(self.services.keys.as_ref(), &lease, &acceptance, now)?;
        let runtime = DomeSessionRuntime::start_with_session_id(
            lease.clone(),
            self.services.keys.as_ref().clone(),
            &instance,
            &preset,
            &session_id,
            now,
        )?;
        let new_records = [
            DomeHostingRecordV1::LeaseIssued(lease),
            DomeHostingRecordV1::HostAccepted(acceptance),
            DomeHostingRecordV1::LeaseActivated(activation),
        ];
        for record in &new_records {
            self.persist_dome_hosting_record(&replica, &instance.instance_id, record)
                .await?;
        }
        self.dome_host_sessions
            .lock()
            .await
            .insert(instance.instance_id.clone(), runtime);
        let mut all_records = records;
        all_records.extend(new_records);
        self.publish_dome_hosting_hint(
            &instance.spatial_context,
            &instance.instance_id,
            &session_id,
        )
        .await?;
        self.hosting_view(&instance, &all_records, now).await
    }

    pub async fn prepare_community_node_dome_hosting(
        &self,
        input: PrepareCommunityNodeDomeHostingInput,
    ) -> Result<DomeHostingView> {
        let replica = self.hosting_context_replica(&input.spatial_context).await?;
        let instance = self
            .hosting_instance(&replica, &input.instance_id)
            .await?
            .context("Dome instance was not found")?;
        self.ensure_dome_hosting_owner(&instance)?;
        let mut records = self
            .list_dome_hosting_records(&replica, &input.instance_id)
            .await?;
        let now = Utc::now().timestamp_millis();
        let epoch = next_hosting_epoch(&records);
        let lease = build_signed_dome_hosting_lease(
            self.services.keys.as_ref(),
            DomeHostingLeaseV1 {
                lease_id: format!("lease-{}-{epoch}", instance.instance_id),
                spatial_context: instance.spatial_context.clone(),
                instance_id: instance.instance_id.clone(),
                instance_generation: instance.generation,
                owner_pubkey: instance.owner_pubkey.clone(),
                host: DomeHostTargetV1::CommunityNode {
                    node_id: Pubkey::from(input.node_id),
                    api_base_url: input.api_base_url,
                },
                manifest_blob_hash: instance.preset_ref.manifest_blob_hash.clone(),
                manifest_version: METAVERSE_WORLD_VERSION,
                epoch,
                issued_at: now,
                expires_at: lease_expiry(now, input.lease_duration_millis)?,
            },
        )?;
        let record = DomeHostingRecordV1::LeaseIssued(lease);
        self.persist_dome_hosting_record(&replica, &instance.instance_id, &record)
            .await?;
        records.push(record);
        self.dome_host_sessions
            .lock()
            .await
            .remove(&instance.instance_id);
        self.publish_dome_hosting_hint(
            &instance.spatial_context,
            &instance.instance_id,
            "transfer",
        )
        .await?;
        self.hosting_view(&instance, &records, now).await
    }

    pub async fn activate_community_node_dome_hosting(
        &self,
        input: ActivateCommunityNodeDomeHostingInput,
    ) -> Result<DomeHostingView> {
        let replica = self.hosting_context_replica(&input.spatial_context).await?;
        let instance = self
            .hosting_instance(&replica, &input.instance_id)
            .await?
            .context("Dome instance was not found")?;
        self.ensure_dome_hosting_owner(&instance)?;
        let mut records = self
            .list_dome_hosting_records(&replica, &input.instance_id)
            .await?;
        let lease = current_unique_lease(&records)?.context("no pending Hosting Lease")?;
        if !matches!(lease.lease.host, DomeHostTargetV1::CommunityNode { .. }) {
            anyhow::bail!("pending Hosting Lease does not target a Community Node");
        }
        let acceptance: SignedDomeHostingAcceptanceV1 =
            serde_json::from_str(&input.signed_acceptance_json)
                .context("invalid Community Node hosting acceptance")?;
        let now = Utc::now().timestamp_millis();
        let activation =
            activate_dome_hosting_lease(self.services.keys.as_ref(), &lease, &acceptance, now)?;
        let new_records = [
            DomeHostingRecordV1::HostAccepted(acceptance),
            DomeHostingRecordV1::LeaseActivated(activation),
        ];
        for record in &new_records {
            self.persist_dome_hosting_record(&replica, &instance.instance_id, record)
                .await?;
        }
        records.extend(new_records);
        self.publish_dome_hosting_hint(
            &instance.spatial_context,
            &instance.instance_id,
            "community-node-active",
        )
        .await?;
        self.hosting_view(&instance, &records, now).await
    }

    pub async fn close_dome_hosting(
        &self,
        input: CloseDomeHostingInput,
    ) -> Result<DomeHostingView> {
        let replica = self.hosting_context_replica(&input.spatial_context).await?;
        let instance = self
            .hosting_instance(&replica, &input.instance_id)
            .await?
            .context("Dome instance was not found")?;
        self.ensure_dome_hosting_owner(&instance)?;
        let mut records = self
            .list_dome_hosting_records(&replica, &input.instance_id)
            .await?;
        let lease = current_unique_lease(&records)?.context("no Hosting Lease to close")?;
        let now = Utc::now().timestamp_millis();
        let record = DomeHostingRecordV1::LeaseClosed(close_dome_hosting_lease(
            self.services.keys.as_ref(),
            &lease,
            now,
        )?);
        self.persist_dome_hosting_record(&replica, &instance.instance_id, &record)
            .await?;
        records.push(record);
        self.dome_host_sessions
            .lock()
            .await
            .remove(&instance.instance_id);
        self.publish_dome_hosting_hint(&instance.spatial_context, &instance.instance_id, "closed")
            .await?;
        self.hosting_view(&instance, &records, now).await
    }

    pub async fn submit_dome_session_input(
        &self,
        input: SubmitDomeSessionInput,
    ) -> Result<SignedDomePhysicsSnapshotV1> {
        let now = Utc::now().timestamp_millis();
        let mut sessions = self.dome_host_sessions.lock().await;
        let runtime = sessions
            .get_mut(&input.instance_id)
            .context("this device is not the active Dome host")?;
        if runtime.lease().spatial_context != input.spatial_context {
            anyhow::bail!("Dome session input SpatialContext mismatch");
        }
        let signed = build_signed_dome_session_input(
            self.services.keys.as_ref(),
            kukuri_core::DomeSessionInputV1 {
                input_id: format!("input-{}-{}", input.instance_id, input.sequence),
                instance_id: input.instance_id,
                instance_generation: runtime.lease().instance_generation,
                lease_epoch: runtime.lease().epoch,
                session_id: runtime.session_id().to_string(),
                participant_pubkey: self.services.keys.public_key(),
                sequence: input.sequence,
                sent_at: now,
                input: input.input,
            },
        )?;
        runtime.apply_signed_input(&signed)?;
        runtime.signed_snapshot(now)
    }

    async fn hosting_context_replica(&self, context: &SpatialContextV1) -> Result<ReplicaId> {
        self.ensure_topic_subscription(context.topic_id().as_str())
            .await?;
        let replica = match context {
            SpatialContextV1::Topic { topic_id } => topic_replica_id(topic_id.as_str()),
            SpatialContextV1::Channel {
                topic_id,
                channel_id,
            } => {
                let state = self
                    .private_channel_write_state(topic_id.as_str(), channel_id)
                    .await?;
                current_private_channel_replica_id(&state)
            }
        };
        self.services.docs_sync.open_replica(&replica).await?;
        Ok(replica)
    }

    async fn hosting_instance(
        &self,
        replica: &ReplicaId,
        instance_id: &str,
    ) -> Result<Option<DomeInstanceManifestV1>> {
        let states = self
            .services
            .docs_sync
            .query_replica(
                replica,
                DocQuery::Prefix(stable_key("metaverse/dome-instances", "")),
            )
            .await?;
        for record in states {
            if !record.key.ends_with("/state") {
                continue;
            }
            let state: DomeInstanceStateDocV1 = serde_json::from_slice(&record.value)?;
            if state.instance_id != instance_id {
                continue;
            }
            let Some((_, manifest)) = self
                .fetch_dome_instance_manifest(replica, &state.owner_pubkey)
                .await?
            else {
                continue;
            };
            return Ok(Some(manifest));
        }
        Ok(None)
    }

    async fn list_dome_hosting_records(
        &self,
        replica: &ReplicaId,
        instance_id: &str,
    ) -> Result<Vec<DomeHostingRecordV1>> {
        let prefix = stable_key(HOSTING_RECORD_PREFIX, &format!("{instance_id}/"));
        let records = self
            .services
            .docs_sync
            .query_replica(replica, DocQuery::Prefix(prefix))
            .await?;
        records
            .into_iter()
            .map(|record| serde_json::from_slice(&record.value).map_err(Into::into))
            .collect()
    }

    async fn persist_dome_hosting_record(
        &self,
        replica: &ReplicaId,
        instance_id: &str,
        record: &DomeHostingRecordV1,
    ) -> Result<()> {
        let (epoch, stage, envelope_id) = hosting_record_identity(record);
        self.services
            .docs_sync
            .apply_doc_op(
                replica,
                DocOp::SetJson {
                    key: stable_key(
                        HOSTING_RECORD_PREFIX,
                        &format!("{instance_id}/{epoch:020}/{stage}/{envelope_id}"),
                    ),
                    value: serde_json::to_value(record)?,
                },
            )
            .await
    }

    async fn hosting_view(
        &self,
        instance: &DomeInstanceManifestV1,
        records: &[DomeHostingRecordV1],
        now: i64,
    ) -> Result<DomeHostingView> {
        let (local_heartbeat, participants, sleeping) = {
            let sessions = self.dome_host_sessions.lock().await;
            match sessions.get(&instance.instance_id) {
                Some(runtime) => (
                    Some(now),
                    runtime.participant_count().try_into().unwrap_or(u32::MAX),
                    runtime.is_sleeping(),
                ),
                None => (None, 0, true),
            }
        };
        let state = resolve_dome_hosting_state(instance, records, now, local_heartbeat)?;
        self.services
            .projection_store
            .upsert_dome_hosting_projection(DomeHostingProjectionRow {
                instance_id: instance.instance_id.clone(),
                context_id: instance.spatial_context.canonical_id(),
                topic_id: instance.spatial_context.topic_id().as_str().to_string(),
                channel_id: instance
                    .spatial_context
                    .channel_id()
                    .map(|channel_id| channel_id.as_str().to_string())
                    .unwrap_or_default(),
                state_json: serde_json::to_string(&state)?,
                lease_epoch: state.lease_epoch,
                session_id: state.session_id.clone(),
                derived_at: now,
                projection_version: 1,
            })
            .await?;
        let lease = current_unique_lease(records)?;
        let epoch = lease.as_ref().map(|signed| signed.lease.epoch);
        let activation = epoch.and_then(|epoch| {
            records.iter().rev().find_map(|record| match record {
                DomeHostingRecordV1::LeaseActivated(signed)
                    if signed.activation.lease_epoch == epoch =>
                {
                    Some(signed)
                }
                _ => None,
            })
        });
        let close = epoch.and_then(|epoch| {
            records.iter().rev().find_map(|record| match record {
                DomeHostingRecordV1::LeaseClosed(signed) if signed.close.lease_epoch == epoch => {
                    Some(signed)
                }
                _ => None,
            })
        });
        let preset = self
            .fetch_dome_preset_manifest(&instance.preset_ref)
            .await?
            .context("Dome preset manifest is unavailable")?;
        Ok(DomeHostingView {
            instance_id: instance.instance_id.clone(),
            state,
            signed_lease_json: lease.as_ref().map(serde_json::to_string).transpose()?,
            lease: lease.map(|signed| signed.lease),
            signed_activation_json: activation.map(serde_json::to_string).transpose()?,
            signed_close_json: close.map(serde_json::to_string).transpose()?,
            instance_manifest_json: serde_json::to_string(instance)?,
            preset_manifest_json: serde_json::to_string(&preset)?,
            participants,
            sleeping,
        })
    }

    fn ensure_dome_hosting_owner(&self, instance: &DomeInstanceManifestV1) -> Result<()> {
        if instance.status != DomeInstanceStatusV1::Active
            || instance.relationship_detach.is_some()
            || instance.owner_pubkey.as_str() != self.current_author_pubkey()
        {
            anyhow::bail!("only the active attached Dome owner can change hosting");
        }
        Ok(())
    }

    async fn publish_dome_hosting_hint(
        &self,
        context: &SpatialContextV1,
        instance_id: &str,
        session_id: &str,
    ) -> Result<()> {
        self.services
            .hint_transport
            .publish_hint(
                &channel_hint_topic_for(context.topic_id().as_str(), context.channel_id()),
                GossipHint::SessionChanged {
                    topic_id: context.topic_id().clone(),
                    session_id: session_id.to_string(),
                    object_kind: format!("dome-hosting:{instance_id}"),
                },
            )
            .await
    }
}

fn lease_expiry(now: i64, requested_duration: i64) -> Result<i64> {
    let duration = if requested_duration == 0 {
        DEFAULT_LEASE_MILLIS
    } else {
        requested_duration
    };
    if duration <= 0 || duration > DOME_HOSTING_MAX_LEASE_MILLIS {
        anyhow::bail!("Dome Hosting Lease duration is outside the supported range");
    }
    now.checked_add(duration)
        .context("Dome Hosting Lease expiry overflow")
}

fn next_hosting_epoch(records: &[DomeHostingRecordV1]) -> u64 {
    records
        .iter()
        .map(|record| match record {
            DomeHostingRecordV1::LeaseIssued(signed) => signed.lease.epoch,
            DomeHostingRecordV1::HostAccepted(signed) => signed.acceptance.lease_epoch,
            DomeHostingRecordV1::LeaseActivated(signed) => signed.activation.lease_epoch,
            DomeHostingRecordV1::LeaseClosed(signed) => signed.close.lease_epoch,
        })
        .max()
        .unwrap_or(0)
        .saturating_add(1)
}

fn current_unique_lease(
    records: &[DomeHostingRecordV1],
) -> Result<Option<SignedDomeHostingLeaseV1>> {
    let highest = records
        .iter()
        .filter_map(|record| match record {
            DomeHostingRecordV1::LeaseIssued(signed) => Some(signed.lease.epoch),
            _ => None,
        })
        .max();
    let Some(highest) = highest else {
        return Ok(None);
    };
    let leases = records
        .iter()
        .filter_map(|record| match record {
            DomeHostingRecordV1::LeaseIssued(signed) if signed.lease.epoch == highest => {
                Some(signed.clone())
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    let digests = leases
        .iter()
        .map(|lease| kukuri_core::dome_hosting_lease_digest(&lease.lease))
        .collect::<Result<BTreeSet<_>>>()?;
    if digests.len() > 1 {
        anyhow::bail!("split-brain Hosting Leases require a higher owner epoch");
    }
    Ok(leases.into_iter().next())
}

fn hosting_record_identity(record: &DomeHostingRecordV1) -> (u64, &'static str, &str) {
    match record {
        DomeHostingRecordV1::LeaseIssued(signed) => {
            (signed.lease.epoch, "issued", signed.envelope.id.as_str())
        }
        DomeHostingRecordV1::HostAccepted(signed) => (
            signed.acceptance.lease_epoch,
            "accepted",
            signed.envelope.id.as_str(),
        ),
        DomeHostingRecordV1::LeaseActivated(signed) => (
            signed.activation.lease_epoch,
            "activated",
            signed.envelope.id.as_str(),
        ),
        DomeHostingRecordV1::LeaseClosed(signed) => (
            signed.close.lease_epoch,
            "closed",
            signed.envelope.id.as_str(),
        ),
    }
}
