use super::*;
use kukuri_core::{BlobHash, DomePresetRefV1};

#[derive(Default)]
struct DelayedPresetBlob {
    inner: MemoryBlobService,
    held_hash: TokioMutex<Option<BlobHash>>,
}

#[async_trait]
impl BlobService for DelayedPresetBlob {
    async fn put_blob(&self, bytes: Vec<u8>, mime: &str) -> Result<StoredBlob> {
        self.inner.put_blob(bytes, mime).await
    }
    async fn fetch_blob(&self, hash: &BlobHash) -> Result<Option<Vec<u8>>> {
        if self.held_hash.lock().await.as_ref() == Some(hash) {
            return Ok(None);
        }
        self.inner.fetch_blob(hash).await
    }
    async fn pin_blob(&self, hash: &BlobHash) -> Result<()> {
        self.inner.pin_blob(hash).await
    }
    async fn blob_status(&self, hash: &BlobHash) -> Result<BlobStatus> {
        self.inner.blob_status(hash).await
    }
    async fn local_blob_status(&self, hash: &BlobHash) -> Result<BlobStatus> {
        self.inner.local_blob_status(hash).await
    }
    async fn import_peer_ticket(&self, ticket: &str) -> Result<()> {
        self.inner.import_peer_ticket(ticket).await
    }
}

const TOPIC: &str = "kukuri:topic:pending-dome-preset";

#[tokio::test]
async fn owner_can_manage_and_delete_without_preset_bytes() {
    let f = fixture().await;
    *f.blobs.held_hash.lock().await = Some(BlobHash::new(f.preset.manifest_blob_hash.clone()));
    let rooms = f.app.list_game_rooms(TOPIC).await.unwrap();
    assert!(
        rooms.iter().any(|r| r.room_id == f.dome_id),
        "owner management must not depend on Preset delivery"
    );
    f.app
        .delete_dome(crate::DeleteDomeInput {
            spatial_context: kukuri_core::SpatialContextV1::Topic {
                topic_id: TopicId::new(TOPIC),
            },
            instance_id: f.dome_id.clone(),
            expected_generation: 1,
            operation_id: "missing-preset".into(),
        })
        .await
        .unwrap();
    assert!(
        f.app
            .list_game_rooms(TOPIC)
            .await
            .unwrap()
            .iter()
            .all(|r| r.room_id != f.dome_id)
    );
}

#[tokio::test]
async fn owner_can_delete_without_derived_game_manifest() {
    let f = fixture().await;
    let row = f
        .store
        .list_topic_game_rooms(TOPIC)
        .await
        .unwrap()
        .into_iter()
        .find(|r| r.room_id == f.dome_id)
        .unwrap();
    *f.blobs.held_hash.lock().await = Some(row.manifest_blob_hash);
    let mut handles = f.app.services.clone();
    let fresh_store = Arc::new(MemoryStore::default());
    handles.store = fresh_store.clone();
    handles.projection_store = fresh_store;
    let reader = AppService::from_handles(handles);
    let rooms = reader.list_game_rooms(TOPIC).await.unwrap();
    assert!(
        rooms.iter().any(|r| r.room_id == f.dome_id),
        "canonical owner metadata survives a missing derived game blob/cache"
    );

    f.app
        .delete_dome(crate::DeleteDomeInput {
            spatial_context: kukuri_core::SpatialContextV1::Topic {
                topic_id: TopicId::new(TOPIC),
            },
            instance_id: f.dome_id,
            expected_generation: 1,
            operation_id: "missing-projection".into(),
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn missing_other_instance_keeps_owner_management_topology_available() {
    let f = fixture().await;
    let instance = f
        .app
        .fetch_dome_instance_manifest(&topic_replica_id(TOPIC), &f.app.keys().public_key())
        .await
        .unwrap()
        .unwrap();
    let mut other_handles = f.app.services.clone();
    other_handles.keys = Arc::new(generate_keys());
    let other = AppService::from_handles(other_handles);
    let owned = other
        .create_metaverse_room(
            TOPIC,
            CreateMetaverseRoomInput {
                title: "Available owner".into(),
                description: String::new(),
                max_peers: Some(8),
            },
        )
        .await
        .unwrap();
    *f.blobs.held_hash.lock().await = Some(instance.0.current_manifest.hash);
    let context = kukuri_core::SpatialContextV1::Topic {
        topic_id: TopicId::new(TOPIC),
    };
    let topology = other
        .list_dome_connection_topology(context)
        .await
        .expect("missing remote manifest must not block owner management");
    assert!(
        topology
            .resolution
            .topology
            .components
            .iter()
            .any(|component| component.instance_ids.contains(&owned))
    );
}

struct Fixture {
    app: AppService,
    docs: Arc<MemoryDocsSync>,
    blobs: Arc<DelayedPresetBlob>,
    store: Arc<MemoryStore>,
    dome_id: String,
    game_id: String,
    preset: DomePresetRefV1,
}

async fn fixture() -> Fixture {
    let store = Arc::new(MemoryStore::default());
    let docs = Arc::new(MemoryDocsSync::default());
    let blobs = Arc::new(DelayedPresetBlob::default());
    let transport = Arc::new(FakeTransport::new("listing", FakeNetwork::default()));
    let app = app_service_from_dependencies(
        store.clone(),
        store.clone(),
        transport.clone(),
        transport,
        docs.clone(),
        blobs.clone(),
        generate_keys(),
    );
    let dome_id = app
        .create_metaverse_room(
            TOPIC,
            CreateMetaverseRoomInput {
                title: "Dome awaiting preset".into(),
                description: String::new(),
                max_peers: Some(4),
            },
        )
        .await
        .expect("create Dome");
    let game_id = app
        .create_game_room(
            TOPIC,
            CreateGameRoomInput {
                title: "available game".into(),
                description: String::new(),
                participants: vec!["Alice".into(), "Bob".into()],
            },
        )
        .await
        .expect("create game");
    let preset = app
        .list_game_rooms(TOPIC)
        .await
        .expect("initial rooms")
        .into_iter()
        .find(|room| room.room_id == dome_id)
        .expect("Dome row")
        .metaverse
        .expect("Dome state")
        .preset_ref;
    Fixture {
        app,
        docs,
        blobs,
        store,
        dome_id,
        game_id,
        preset,
    }
}

fn preset_key(preset: &DomePresetRefV1) -> String {
    format!(
        "metaverse/dome-presets/{}/revisions/{:020}",
        preset.preset_id, preset.revision
    )
}

async fn assert_pending_then_available(f: &Fixture) {
    let rooms = f
        .app
        .list_game_rooms(TOPIC)
        .await
        .expect("pending preset must not fail room listing");
    assert!(rooms.iter().any(|r| r.room_id == f.game_id));
    let owned = rooms
        .iter()
        .find(|r| r.room_id == f.dome_id)
        .expect("owner management remains available");
    assert_eq!(owned.phase_label.as_deref(), Some("management_only"));
    // Pending is a read result, not removal of the canonical/projection record.
    assert_eq!(
        f.store
            .list_topic_game_rooms(TOPIC)
            .await
            .expect("stored rooms")
            .len(),
        2
    );
}

#[tokio::test]
async fn missing_preset_state_keeps_other_rooms_and_recovers_after_delivery() {
    let f = fixture().await;
    let replica = author_replica_id(f.preset.owner_pubkey.as_str());
    let key = preset_key(&f.preset);
    let record = f
        .docs
        .query_replica(&replica, DocQuery::Exact(key.clone()))
        .await
        .expect("preset state")
        .pop()
        .expect("state record");
    f.docs
        .apply_doc_op(
            &replica,
            DocOp::DeletePrefix {
                prefix: key.clone(),
            },
        )
        .await
        .expect("delay state");
    assert_pending_then_available(&f).await;
    f.docs
        .apply_doc_op(
            &replica,
            DocOp::SetBytes {
                key,
                value: record.value,
            },
        )
        .await
        .expect("deliver state");
    let rooms = f.app.list_game_rooms(TOPIC).await.expect("delivered rooms");
    assert_eq!(rooms.len(), 2);
    assert!(
        rooms
            .iter()
            .any(|room| room.room_id == f.dome_id && room.dome_hosting.is_some())
    );
}

#[tokio::test]
async fn missing_preset_blob_keeps_other_rooms_and_recovers_after_delivery() {
    let f = fixture().await;
    *f.blobs.held_hash.lock().await = Some(BlobHash::new(f.preset.manifest_blob_hash.clone()));
    assert_pending_then_available(&f).await;
    *f.blobs.held_hash.lock().await = None;
    let rooms = f.app.list_game_rooms(TOPIC).await.expect("delivered rooms");
    assert_eq!(rooms.len(), 2);
    assert!(
        rooms
            .iter()
            .any(|room| room.room_id == f.dome_id && room.dome_hosting.is_some())
    );
}

#[tokio::test]
async fn mismatched_preset_reference_is_an_error_not_pending() {
    let f = fixture().await;
    let replica = author_replica_id(f.preset.owner_pubkey.as_str());
    let key = preset_key(&f.preset);
    let record = f
        .docs
        .query_replica(&replica, DocQuery::Exact(key.clone()))
        .await
        .expect("preset state")
        .pop()
        .expect("state record");
    let mut state: serde_json::Value = serde_json::from_slice(&record.value).expect("decode");
    state["revision"] = serde_json::json!(f.preset.revision + 1);
    f.docs
        .apply_doc_op(&replica, DocOp::SetJson { key, value: state })
        .await
        .expect("mismatch state");
    let error = f
        .app
        .list_game_rooms(TOPIC)
        .await
        .expect_err("invalid reference must fail");
    assert!(error.to_string().contains("does not match its reference"));
}

#[tokio::test]
async fn invalid_preset_signature_is_an_error_not_pending() {
    let f = fixture().await;
    let replica = author_replica_id(f.preset.owner_pubkey.as_str());
    let state_record = f
        .docs
        .query_replica(&replica, DocQuery::Exact(preset_key(&f.preset)))
        .await
        .expect("preset state")
        .pop()
        .expect("state record");
    let state: DomePresetStateDocV1 =
        serde_json::from_slice(&state_record.value).expect("decode state");
    let key = format!("envelopes/{}", state.last_envelope_id.as_str());
    let record = f
        .docs
        .query_replica(&replica, DocQuery::Exact(key.clone()))
        .await
        .expect("envelope")
        .pop()
        .expect("signed envelope");
    let mut envelope: KukuriEnvelope =
        serde_json::from_slice(&record.value).expect("decode envelope");
    envelope.content = "tampered preset".into();
    f.docs
        .apply_doc_op(
            &replica,
            DocOp::SetJson {
                key,
                value: serde_json::to_value(envelope).expect("encode"),
            },
        )
        .await
        .expect("tamper");
    f.app
        .list_game_rooms(TOPIC)
        .await
        .expect_err("invalid signature must fail, not be skipped");
}

#[tokio::test]
async fn missing_preset_envelope_keeps_other_rooms_and_recovers_after_delivery() {
    let f = fixture().await;
    let replica = author_replica_id(f.preset.owner_pubkey.as_str());
    let record = f
        .docs
        .query_replica(&replica, DocQuery::Exact(preset_key(&f.preset)))
        .await
        .expect("state")
        .pop()
        .expect("state record");
    let state: DomePresetStateDocV1 = serde_json::from_slice(&record.value).expect("decode state");
    let key = format!("envelopes/{}", state.last_envelope_id.as_str());
    let signed = f
        .docs
        .query_replica(&replica, DocQuery::Exact(key.clone()))
        .await
        .expect("signed envelope")
        .pop()
        .expect("envelope");
    f.docs
        .apply_doc_op(
            &replica,
            DocOp::DeletePrefix {
                prefix: key.clone(),
            },
        )
        .await
        .expect("delay envelope");
    assert_pending_then_available(&f).await;
    f.docs
        .apply_doc_op(
            &replica,
            DocOp::SetBytes {
                key,
                value: signed.value,
            },
        )
        .await
        .expect("deliver envelope");
    assert_eq!(
        f.app
            .list_game_rooms(TOPIC)
            .await
            .expect("complete rooms")
            .len(),
        2
    );
}

#[tokio::test]
async fn current_instance_preset_controls_readiness_even_with_an_old_cache_row() {
    let f = fixture().await;
    let old_row = f
        .store
        .list_topic_game_rooms(TOPIC)
        .await
        .expect("rows")
        .into_iter()
        .find(|row| row.room_id == f.dome_id)
        .expect("Dome row");
    let mut customization = old_row
        .metaverse
        .as_ref()
        .expect("state")
        .dome
        .customization
        .clone();
    customization.environment.fog_density_micros += 1;
    f.app
        .update_metaverse_room(
            TOPIC,
            &f.dome_id,
            UpdateMetaverseRoomInput {
                status: old_row.status.clone(),
                customization,
            },
        )
        .await
        .expect("new revision");
    let current = f
        .app
        .list_game_rooms(TOPIC)
        .await
        .expect("new rooms")
        .into_iter()
        .find(|row| row.room_id == f.dome_id)
        .expect("new Dome")
        .metaverse
        .expect("new state");
    assert_eq!(current.preset_ref.revision, f.preset.revision + 1);
    f.store
        .upsert_game_room_cache(old_row)
        .await
        .expect("lagging derived cache");
    *f.blobs.held_hash.lock().await = Some(BlobHash::new(current.preset_ref.manifest_blob_hash));
    assert_pending_then_available(&f).await;
    *f.blobs.held_hash.lock().await = None;
    assert_eq!(
        f.app
            .list_game_rooms(TOPIC)
            .await
            .expect("complete rooms")
            .len(),
        2
    );
}
