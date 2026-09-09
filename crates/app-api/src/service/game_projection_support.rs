//! ScoreGame の canonical state と projection の書込み順を揃える。

use super::*;
use std::sync::Weak;
use tokio::sync::OwnedMutexGuard;

/// Store の一意キー(room_id)と同じ単位。handles の clone 間でも共有する。
#[derive(Default)]
pub(crate) struct GameRoomProjectionLocks {
    rooms: Mutex<HashMap<String, Weak<Mutex<()>>>>,
}

impl GameRoomProjectionLocks {
    pub(crate) async fn lock(&self, room_id: &str) -> OwnedMutexGuard<()> {
        let lock = {
            let mut rooms = self.rooms.lock().await;
            rooms.retain(|_, lock| lock.strong_count() > 0);
            match rooms.get(room_id).and_then(Weak::upgrade) {
                Some(lock) => lock,
                None => {
                    let lock = Arc::new(Mutex::new(()));
                    rooms.insert(room_id.to_string(), Arc::downgrade(&lock));
                    lock
                }
            }
        };
        lock.lock_owned().await
    }
}

pub(crate) async fn hydrate_game_room_from_record(
    services: &ServiceHandles,
    topic_id: &str,
    replica: &ReplicaId,
    mut record: DocRecord,
) -> Result<bool> {
    // A moving docs pointer may invalidate more than one fetch. Bound re-resolution;
    // false keeps the existing caller's retry/recovery path, rather than claiming success.
    for _ in 0..session_projection_retry_attempts() {
        let state: GameRoomStateDocV1 = serde_json::from_slice(&record.value)?;
        services
            .projection_store
            .mark_blob_status(
                &state.current_manifest.hash,
                blob_status(
                    services
                        .blob_service
                        .blob_status(&state.current_manifest.hash)
                        .await?,
                ),
            )
            .await?;
        let Some(manifest) = fetch_manifest_blob::<GameRoomManifestBlobV1>(
            services.blob_service.as_ref(),
            &state.current_manifest,
        )
        .await?
        else {
            return Ok(false);
        };
        // Slow blob I/O precedes the room lock. Other rooms never wait for it.
        // Metaverse keeps its existing projection/lifecycle behavior.
        let projection_guard = if manifest.room_kind == GameRoomKind::ScoreGame {
            Some(services.game_room_projections.lock(&state.room_id).await)
        } else {
            None
        };
        let row = game_projection_row_from_state(&state, &manifest, topic_id, replica);
        if projection_guard.is_some() {
            let Some(current) = query_replica_local_only(
                services.docs_sync.as_ref(),
                replica,
                DocQuery::Exact(record.key.clone()),
            )
            .await?
            .into_iter()
            .next() else {
                return Ok(false);
            };
            if current.value != record.value {
                // Neither timestamps nor hash ordering decide freshness: docs does.
                // Drop this guard before fetching the current candidate's blob.
                record = current;
                continue;
            }
            if let Some(mut cached) = services
                .projection_store
                .list_topic_game_rooms(topic_id)
                .await?
                .into_iter()
                .find(|cached| cached.room_id == row.room_id)
            {
                cached.derived_at = row.derived_at;
                if cached == row {
                    return Ok(true);
                }
            }
        }
        // For ScoreGame, canonical comparison and commit share the local writer lock.
        services
            .projection_store
            .upsert_game_room_cache(row)
            .await?;
        return Ok(true);
    }
    Ok(false)
}
