use super::{
    DocFetchPolicy, DocQuery, DocRecord, IndexScopeKind, IngestPipeline, KukuriEnvelope, ReplicaId,
    source::{MediaScanTarget, PostObjectView, SourceResolver},
    verify_post_withdrawal,
};
use anyhow::{Context, Result, ensure};
use async_trait::async_trait;
use kukuri_cn_safety::{ScanError, provider::ScanReferenceGuard};
use std::sync::Mutex;

pub(super) struct ReferenceGuard<'a> {
    pub pipeline: &'a IngestPipeline,
    pub scope_kind: IndexScopeKind,
    pub scope_id: &'a str,
    pub replica: &'a ReplicaId,
    pub object: &'a PostObjectView,
    pub record: &'a DocRecord,
    pub envelope: Option<&'a DocRecord>,
    pub media_targets: Mutex<Option<Vec<MediaScanTarget>>>,
}

impl ReferenceGuard<'_> {
    async fn check_current(&self) -> Result<()> {
        ensure!(
            !self.object.object_id.trim().is_empty()
                && !self.object.author.trim().is_empty()
                && self.record.key == format!("objects/{}/state", self.object.object_id),
            "invalid post reference"
        );
        ensure!(
            self.pipeline
                .entries
                .is_scope_supported(self.scope_kind, self.scope_id)
                .await?,
            "scope is no longer supported"
        );
        ensure!(
            !self
                .pipeline
                .entries
                .is_transmission_prevented(&self.object.object_id)
                .await?,
            "post is transmission prevented"
        );
        let state = self
            .pipeline
            .docs_sync
            .query_replica_with_policy(
                self.replica,
                DocQuery::Exact(self.record.key.clone()),
                DocFetchPolicy::LocalOnly,
            )
            .await?;
        ensure!(
            state
                .iter()
                .any(|current| current.content_hash == self.record.content_hash),
            "post state changed during scan"
        );
        {
            let original = self.envelope.context("signed post envelope missing")?;
            let envelope: KukuriEnvelope = serde_json::from_slice(&original.value)?;
            envelope.verify()?;
            let content = envelope
                .post_content()?
                .context("source is not a signed post")?;
            ensure!(
                content.payload_ref == self.object.payload_ref
                    && content.attachments == self.object.attachments
                    && content.media_manifest_refs == self.object.media_manifest_refs,
                "materialized post differs from signed content"
            );
            match self.scope_kind {
                IndexScopeKind::PublicTopic => ensure!(
                    content.topic_id.as_str() == self.scope_id
                        && content.channel_id.is_none()
                        && content.visibility == kukuri_core::ObjectVisibility::Public,
                    "post does not belong to public scope"
                ),
                IndexScopeKind::PrivateChannel => ensure!(
                    content
                        .channel_id
                        .as_ref()
                        .is_some_and(|id| id.as_str() == self.scope_id),
                    "post does not belong to private scope"
                ),
            }

            ensure!(
                envelope.id.as_str() == self.object.object_id
                    && envelope.pubkey.as_str() == self.object.author,
                "post envelope identity changed"
            );
            let current = self
                .pipeline
                .docs_sync
                .query_replica_with_policy(
                    self.replica,
                    DocQuery::Exact(original.key.clone()),
                    DocFetchPolicy::LocalOnly,
                )
                .await?;
            ensure!(
                current
                    .iter()
                    .any(|record| record.content_hash == original.content_hash),
                "post envelope changed during scan"
            );
            let withdrawals = self
                .pipeline
                .docs_sync
                .query_replica_with_policy(
                    self.replica,
                    DocQuery::Exact(format!("withdrawals/{}/state", self.object.object_id)),
                    DocFetchPolicy::LocalOnly,
                )
                .await?;
            for record in withdrawals {
                if let Ok(withdrawal) = serde_json::from_slice::<KukuriEnvelope>(&record.value) {
                    ensure!(
                        verify_post_withdrawal(&withdrawal, &envelope).is_err(),
                        "post was withdrawn during scan"
                    );
                }
            }
        }
        let expected = {
            self.media_targets
                .lock()
                .expect("media reference guard mutex")
                .clone()
        };
        if let Some(expected) = expected {
            let source = SourceResolver::new(
                self.pipeline.docs_sync.as_ref(),
                self.pipeline.blob_service.as_deref(),
            );
            ensure!(
                source.media_scan_targets(self.replica, self.object).await? == expected,
                "media manifest changed during scan"
            );
        }
        Ok(())
    }
}

#[async_trait]
impl ScanReferenceGuard for ReferenceGuard<'_> {
    async fn check(&self) -> Result<(), ScanError> {
        self.check_current()
            .await
            .map_err(|_| ScanError::Unavailable("scan reference is no longer valid".into()))
    }
}
