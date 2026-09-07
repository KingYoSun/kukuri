use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use async_trait::async_trait;
use kukuri_blob_service::{BlobService, BlobStatus, StoredBlob};
use kukuri_cn_core::{IndexEntryStore, IndexScopeKind, MemoryIndexEntryStore, NewIndexEntry};
use kukuri_cn_indexer::ingest::{IngestPipeline, IngestSummary};
use kukuri_cn_indexer::projection::{IndexProjection, IndexedEntry, MemoryIndexProjection};
use kukuri_cn_safety::provider::{ProviderScanRequest, ProviderScanResult, ScanError};
use kukuri_cn_safety::{
    MockSafetyProvider, ModerationEventSigner, SafetyProvider, SafetyProviderCapability,
};
use kukuri_cn_safety_runtime::clock::SystemScanClock;
use kukuri_cn_safety_runtime::id::UuidEventIdGenerator;
use kukuri_cn_safety_runtime::{
    MemorySafetyArtifactStore, SafetyOrchestrator, SafetyScanService,
    Secp256k1ModerationEventSigner,
};
use kukuri_core::{
    AssetRef, BlobHash, KukuriEnvelope, KukuriKeys, ObjectVisibility, PayloadRef, ReplicaId,
    TopicId, blob_hash, build_post_envelope_with_payload,
};
use kukuri_docs_sync::{
    DocOp, DocsSync, MemoryDocsSync, private_channel_replica_id, topic_replica_id,
};

pub(super) const SCOPE: &str = "source-contract";

#[derive(Clone, Default)]
pub(super) struct Observed {
    pub ephemeral: Vec<String>,
    pub durable: usize,
    pub puts: usize,
    pub pins: usize,
    pub scans: Vec<ProviderScanRequest>,
    pub entry_upserts: Vec<String>,
    pub entry_removes: Vec<String>,
    pub projection_upserts: Vec<String>,
    pub projection_removes: Vec<String>,
    pub scope_removes: usize,
}

type Trace = Arc<Mutex<Observed>>;

#[derive(Clone)]
pub(super) enum Reply {
    Bytes(Vec<u8>),
    Missing,
    Failed,
}

pub(super) struct ObservedBlobs {
    trace: Trace,
    replies: Mutex<HashMap<String, Reply>>,
}
impl ObservedBlobs {
    pub fn respond(&self, hash: &BlobHash, reply: Reply) {
        self.replies
            .lock()
            .unwrap()
            .insert(hash.as_str().into(), reply);
    }
    fn read(&self, hash: &BlobHash) -> Result<Option<Vec<u8>>> {
        match self
            .replies
            .lock()
            .unwrap()
            .get(hash.as_str())
            .cloned()
            .unwrap_or(Reply::Missing)
        {
            Reply::Bytes(bytes) => Ok(Some(bytes)),
            Reply::Missing => Ok(None),
            Reply::Failed => anyhow::bail!("fixture fetch unavailable"),
        }
    }
}
#[async_trait]
impl BlobService for ObservedBlobs {
    async fn put_blob(&self, data: Vec<u8>, mime: &str) -> Result<StoredBlob> {
        self.trace.lock().unwrap().puts += 1;
        let hash = blob_hash(&data);
        let bytes = data.len() as u64;
        self.respond(&hash, Reply::Bytes(data));
        Ok(StoredBlob {
            hash,
            mime: mime.into(),
            bytes,
        })
    }
    async fn fetch_blob(&self, hash: &BlobHash) -> Result<Option<Vec<u8>>> {
        self.trace.lock().unwrap().durable += 1;
        self.read(hash)
    }
    async fn fetch_blob_ephemeral(&self, hash: &BlobHash) -> Result<Option<Vec<u8>>> {
        self.trace
            .lock()
            .unwrap()
            .ephemeral
            .push(hash.as_str().into());
        self.read(hash)
    }
    async fn pin_blob(&self, _hash: &BlobHash) -> Result<()> {
        self.trace.lock().unwrap().pins += 1;
        Ok(())
    }
    async fn blob_status(&self, _hash: &BlobHash) -> Result<BlobStatus> {
        Ok(BlobStatus::Available)
    }
    async fn import_peer_ticket(&self, _ticket: &str) -> Result<()> {
        Ok(())
    }
}

struct ObservedProvider {
    trace: Trace,
    inner: MockSafetyProvider,
}
#[async_trait]
impl SafetyProvider for ObservedProvider {
    fn name(&self) -> &str {
        self.inner.name()
    }
    fn capabilities(&self) -> &[SafetyProviderCapability] {
        self.inner.capabilities()
    }
    async fn scan(&self, request: &ProviderScanRequest) -> Result<ProviderScanResult, ScanError> {
        self.trace.lock().unwrap().scans.push(request.clone());
        self.inner.scan(request).await
    }
}

pub(super) struct ObservedEntries {
    pub inner: MemoryIndexEntryStore,
    trace: Trace,
}
#[async_trait]
impl IndexEntryStore for ObservedEntries {
    async fn upsert_entry(&self, entry: &NewIndexEntry) -> Result<()> {
        self.trace
            .lock()
            .unwrap()
            .entry_upserts
            .push(entry.object_id.clone());
        self.inner.upsert_entry(entry).await
    }
    async fn remove_entry(&self, kind: IndexScopeKind, scope: &str, id: &str) -> Result<()> {
        self.trace.lock().unwrap().entry_removes.push(id.into());
        self.inner.remove_entry(kind, scope, id).await
    }
    async fn remove_scope(&self, kind: IndexScopeKind, scope: &str) -> Result<()> {
        self.trace.lock().unwrap().scope_removes += 1;
        self.inner.remove_scope(kind, scope).await
    }
    async fn filter_surfaceable(
        &self,
        kind: IndexScopeKind,
        candidates: &[(String, String)],
    ) -> Result<Vec<(String, String)>> {
        self.inner.filter_surfaceable(kind, candidates).await
    }
    async fn list_scopes(&self) -> Result<Vec<(IndexScopeKind, String)>> {
        self.inner.list_scopes().await
    }
    async fn is_transmission_prevented(&self, id: &str) -> Result<bool> {
        self.inner.is_transmission_prevented(id).await
    }
}

pub(super) struct ObservedProjection {
    pub inner: MemoryIndexProjection,
    trace: Trace,
}
#[async_trait]
impl IndexProjection for ObservedProjection {
    async fn upsert_entry(&self, entry: &IndexedEntry) -> Result<()> {
        self.trace
            .lock()
            .unwrap()
            .projection_upserts
            .push(entry.object_id.clone());
        self.inner.upsert_entry(entry).await
    }
    async fn contains_object(&self, kind: IndexScopeKind, scope: &str, id: &str) -> Result<bool> {
        self.inner.contains_object(kind, scope, id).await
    }
    async fn count_scope(&self, kind: IndexScopeKind, scope: &str) -> Result<usize> {
        self.inner.count_scope(kind, scope).await
    }
    async fn remove_scope(&self, kind: IndexScopeKind, scope: &str) -> Result<()> {
        self.trace.lock().unwrap().scope_removes += 1;
        self.inner.remove_scope(kind, scope).await
    }
    async fn remove_object(&self, kind: IndexScopeKind, scope: &str, id: &str) -> Result<()> {
        self.trace
            .lock()
            .unwrap()
            .projection_removes
            .push(id.into());
        self.inner.remove_object(kind, scope, id).await
    }
}

pub(super) struct Fixture {
    pub docs: Arc<MemoryDocsSync>,
    pub blobs: Arc<ObservedBlobs>,
    pub entries: Arc<ObservedEntries>,
    pub projection: Arc<ObservedProjection>,
    pub kind: IndexScopeKind,
    pub replica: ReplicaId,
    service: Arc<SafetyScanService>,
    trace: Trace,
}
impl Fixture {
    pub async fn new(kind: IndexScopeKind) -> Result<Self> {
        let trace = Arc::new(Mutex::new(Observed::default()));
        let signer = Secp256k1ModerationEventSigner::from_secret(&format!("{:064x}", 1))?;
        let artifacts = Arc::new(MemorySafetyArtifactStore::new());
        let provider = Arc::new(ObservedProvider {
            trace: trace.clone(),
            inner: MockSafetyProvider::known_csam("source-contract"),
        });
        let orchestrator = SafetyOrchestrator::builder(
            signer.issuer_node_id(),
            Arc::new(SystemScanClock),
            Arc::new(UuidEventIdGenerator),
        )
        .provider(provider)
        .build()?;
        let service = Arc::new(
            SafetyScanService::builder(Arc::new(orchestrator), artifacts.clone())
                .signer(Arc::new(signer))
                .build()?,
        );
        let docs = Arc::new(MemoryDocsSync::default());
        let replica = match kind {
            IndexScopeKind::PublicTopic => topic_replica_id(SCOPE),
            IndexScopeKind::PrivateChannel => private_channel_replica_id(SCOPE),
        };
        if kind == IndexScopeKind::PrivateChannel {
            docs.register_private_replica_secret(&replica, &"11".repeat(32))
                .await?;
        }
        docs.open_replica(&replica).await?;
        Ok(Self {
            docs,
            replica,
            kind,
            service,
            blobs: Arc::new(ObservedBlobs {
                trace: trace.clone(),
                replies: Mutex::new(HashMap::new()),
            }),
            entries: Arc::new(ObservedEntries {
                inner: MemoryIndexEntryStore::new(artifacts),
                trace: trace.clone(),
            }),
            projection: Arc::new(ObservedProjection {
                inner: MemoryIndexProjection::new(),
                trace: trace.clone(),
            }),
            trace,
        })
    }
    pub fn observed(&self) -> Observed {
        self.trace.lock().unwrap().clone()
    }
    pub fn clear_io(&self) {
        *self.trace.lock().unwrap() = Observed::default();
    }
    pub async fn ingest(&self) -> Result<IngestSummary> {
        // Reconstruct the pipeline to model a later pass/restart with the same durable inputs.
        IngestPipeline::new(
            self.docs.clone(),
            self.service.clone(),
            self.entries.clone(),
            self.projection.clone(),
        )
        .with_blob_service(self.blobs.clone())
        .ingest_scope(self.kind, SCOPE, &self.replica)
        .await
    }
    pub fn post(&self, bytes: &[u8]) -> Result<Post> {
        let hash = blob_hash(bytes);
        self.blobs.respond(&hash, Reply::Bytes(bytes.to_vec()));
        self.post_with(
            PayloadRef::BlobText {
                hash,
                mime: "text/markdown".into(),
                bytes: bytes.len() as u64,
            },
            Vec::new(),
            Vec::new(),
        )
    }
    pub fn post_with(
        &self,
        payload: PayloadRef,
        attachments: Vec<AssetRef>,
        refs: Vec<String>,
    ) -> Result<Post> {
        let keys = KukuriKeys::generate();
        let visibility = if self.kind == IndexScopeKind::PrivateChannel {
            ObjectVisibility::Private
        } else {
            ObjectVisibility::Public
        };
        let envelope = build_post_envelope_with_payload(
            &keys,
            &TopicId::new(SCOPE),
            payload,
            attachments,
            refs,
            None,
            visibility,
        )?;
        let state = serde_json::to_value(envelope.to_post_object()?.expect("post"))?;
        Ok(Post {
            id: envelope.id.as_str().into(),
            keys,
            envelope,
            state,
        })
    }
    pub async fn persist(&self, post: &Post) -> Result<()> {
        self.set(&format!("objects/{}/state", post.id), post.state.clone())
            .await?;
        self.set(
            &format!("objects/{}/envelope", post.id),
            serde_json::to_value(&post.envelope)?,
        )
        .await
    }
    pub async fn set(&self, key: &str, value: serde_json::Value) -> Result<()> {
        self.docs
            .apply_doc_op(
                &self.replica,
                DocOp::SetJson {
                    key: key.into(),
                    value,
                },
            )
            .await
    }
}
pub(super) struct Post {
    pub id: String,
    pub keys: KukuriKeys,
    pub envelope: KukuriEnvelope,
    pub state: serde_json::Value,
}

pub(super) fn assert_no_durable_blob_io(io: &Observed) {
    assert_eq!((io.durable, io.puts, io.pins), (0, 0, 0));
    assert_eq!(
        io.scope_removes, 0,
        "an entry failure must not remove another scope"
    );
}
