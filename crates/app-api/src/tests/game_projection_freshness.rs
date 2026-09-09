use super::*;
use crate::service::{hydrate_game_room_from_key, hydrate_game_rooms_from_replica};
use kukuri_docs_sync::DocFetchPolicy;
use kukuri_store::GameRoomProjectionRow;
use tokio::sync::Notify;

const TOPIC: &str = "kukuri:topic:game-projection-freshness";

#[derive(Default)]
struct FetchGate {
    entered: Notify,
    release: Notify,
}

#[derive(Default)]
struct GatedBlobService {
    inner: MemoryBlobService,
    gate: TokioMutex<Option<(kukuri_core::BlobHash, Arc<FetchGate>)>>,
}

impl GatedBlobService {
    async fn pause_next_fetch(&self, hash: kukuri_core::BlobHash) -> Arc<FetchGate> {
        let gate = Arc::new(FetchGate::default());
        *self.gate.lock().await = Some((hash, gate.clone()));
        gate
    }
}

#[async_trait]
impl BlobService for GatedBlobService {
    async fn put_blob(&self, data: Vec<u8>, mime: &str) -> Result<StoredBlob> {
        self.inner.put_blob(data, mime).await
    }

    async fn fetch_blob(&self, hash: &kukuri_core::BlobHash) -> Result<Option<Vec<u8>>> {
        let gate = self.gate.lock().await.take_if(|(target, _)| target == hash);
        if let Some((_, gate)) = gate {
            gate.entered.notify_one();
            gate.release.notified().await;
        }
        self.inner.fetch_blob(hash).await
    }

    async fn pin_blob(&self, hash: &kukuri_core::BlobHash) -> Result<()> {
        self.inner.pin_blob(hash).await
    }

    async fn blob_status(&self, hash: &kukuri_core::BlobHash) -> Result<BlobStatus> {
        self.inner.blob_status(hash).await
    }

    async fn import_peer_ticket(&self, ticket: &str) -> Result<()> {
        self.inner.import_peer_ticket(ticket).await
    }
}

struct Fixture {
    app: AppService,
    blobs: Arc<GatedBlobService>,
    _root: tempfile::TempDir,
}

impl Fixture {
    async fn new(sqlite: bool) -> Self {
        let root = tempdir().unwrap();
        let (store, projection_store): (Arc<dyn Store>, Arc<dyn ProjectionStore>) = if sqlite {
            let store = Arc::new(
                SqliteStore::connect_file(root.path().join("game.db"))
                    .await
                    .unwrap(),
            );
            (store.clone(), store)
        } else {
            let store = Arc::new(MemoryStore::default());
            (store.clone(), store)
        };
        let blobs = Arc::new(GatedBlobService::default());
        let app = AppService::from_handles(ServiceHandles::new(
            store,
            projection_store,
            Arc::new(FakeTransport::new("game-fixture", FakeNetwork::default())),
            Arc::new(NoopHintTransport),
            Arc::new(MemoryDocsSync::default()),
            blobs.clone(),
            generate_keys(),
        ));
        // Drive bootstrap/event hydration explicitly so the test owns every writer's order.
        app.gossip_disabled_topics.lock().await.insert(TOPIC.into());
        Self {
            app,
            blobs,
            _root: root,
        }
    }

    async fn create(&self) -> String {
        self.app
            .create_game_room(
                TOPIC,
                CreateGameRoomInput {
                    title: "finals".into(),
                    description: "controlled hydration".into(),
                    participants: vec!["Alice".into(), "Bob".into()],
                },
            )
            .await
            .unwrap()
    }

    async fn row(&self, room_id: &str) -> GameRoomProjectionRow {
        self.app
            .services
            .projection_store
            .list_topic_game_rooms(TOPIC)
            .await
            .unwrap()
            .into_iter()
            .find(|row| row.room_id == room_id)
            .unwrap()
    }

    async fn update(&self, room_id: &str, score: i64) {
        self.app
            .update_game_room(TOPIC, room_id, update_input(score))
            .await
            .unwrap();
    }
}

fn update_input(score: i64) -> UpdateGameRoomInput {
    UpdateGameRoomInput {
        status: GameRoomStatus::Running,
        phase_label: Some("round 1".into()),
        scores: vec![
            GameScoreView {
                participant_id: "participant-1".into(),
                label: "Alice".into(),
                score,
            },
            GameScoreView {
                participant_id: "participant-2".into(),
                label: "Bob".into(),
                score: 0,
            },
        ],
    }
}

async fn late_hydration_keeps_valid_update(sqlite: bool, batch: bool) {
    let fixture = Fixture::new(sqlite).await;
    let room_id = fixture.create().await;
    let old = fixture.row(&room_id).await;
    let gate = fixture
        .blobs
        .pause_next_fetch(old.manifest_blob_hash.clone())
        .await;
    let services = fixture.app.services.clone();
    let key = old.source_key.clone();
    let hydration = tokio::spawn(async move {
        let replica = topic_replica_id(TOPIC);
        if batch {
            hydrate_game_rooms_from_replica(
                services.docs_sync.as_ref(),
                services.blob_service.as_ref(),
                services.projection_store.as_ref(),
                TOPIC,
                &replica,
                DocFetchPolicy::LocalOnly,
            )
            .await
        } else {
            hydrate_game_room_from_key(
                services.docs_sync.as_ref(),
                services.blob_service.as_ref(),
                services.projection_store.as_ref(),
                TOPIC,
                &replica,
                &key,
            )
            .await
            .map(usize::from)
        }
    });
    timeout(Duration::from_secs(5), gate.entered.notified())
        .await
        .expect("old record reached the blob fetch gate");
    fixture.update(&room_id, 7).await;
    let before = fixture.row(&room_id).await;
    assert_eq!(before.scores[0].score, 7);
    assert_ne!(old.manifest_blob_hash, before.manifest_blob_hash);
    eprintln!(
        "writer sequence: {} captured {}/{} at {}; update committed {} at {}; resume old hydration ({})",
        if batch { "replica" } else { "record" },
        old.source_replica_id.as_str(),
        old.source_key,
        old.updated_at,
        before.manifest_blob_hash.as_str(),
        before.updated_at,
        old.manifest_blob_hash.as_str(),
    );
    gate.release.notify_one();
    timeout(Duration::from_secs(5), hydration)
        .await
        .expect("old hydration completed")
        .unwrap()
        .unwrap();
    let after = fixture.row(&room_id).await;
    assert_eq!(
        before, after,
        "old hydration must not replace a confirmed valid update"
    );
}

#[tokio::test]
async fn late_record_hydration_keeps_valid_update_memory() {
    late_hydration_keeps_valid_update(false, false).await;
}

#[tokio::test]
async fn late_record_hydration_keeps_valid_update_sqlite() {
    late_hydration_keeps_valid_update(true, false).await;
}

#[tokio::test]
async fn late_replica_hydration_keeps_valid_update_memory() {
    late_hydration_keeps_valid_update(false, true).await;
}

#[tokio::test]
async fn late_replica_hydration_keeps_valid_update_sqlite() {
    late_hydration_keeps_valid_update(true, true).await;
}
