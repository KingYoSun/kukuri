# ADR 0006: Game Room Data Classification

## Status
Accepted

## Feature Data Classification
- Feature 名: game room
- Durable / Transient: Durable
- Canonical Source: `iroh-docs` for current state pointer, `iroh-blobs` for manifest payload
- Replicated?: Yes
- Rebuildable From: `docs + blobs`
- Public Replica / Private Replica / Local Only: Public replica for room state pointer, local projection for room list cache
- Gossip Hint 必要有無: No, docs replication is canonical for room updates
- Blob 必要有無: Yes
- SQLite projection 必要有無: Yes
- 必須 contract:
  - `late_joiner_backfills_game_room_manifest`
  - `restart_restores_game_room_manifest`
  - `game_room_score_update_replicates`
  - `finished_game_room_rejects_updates`
- 必須 scenario:
  - game room panel 導入後に `create room -> update score/status -> restart -> restored score card` を追加する

## Decision
- `docs` の topic replica に `game/<room_id>/state` を置き、current manifest blob ref と状態の index metadata だけを保存する。
- game room manifest 本体は JSON blob として `iroh-blobs` に保存し、score/status 更新のたびに新しい blob hash を払い出す。
- participant は create 時に固定し、v1 では add/remove や owner handoff を許可しない。
- room 更新は owner のみ許可し、`Finished` 遷移後は immutable にする。

## Consequences
- late joiner と restart 後の復元は `docs state + manifest blob` だけで成立しなければならない。
- score/status は docs pointer が指す最新 manifest blob だけで再構築できなければならない。
- v1 では replay/snapshot/game move engine を含めない。
