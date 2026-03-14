# ADR 0004: Video Post Data Classification

## Status
Accepted

## Feature Data Classification
- Feature 名: video post
- Durable / Transient: Durable
- Canonical Source: `iroh-docs` for post header / asset refs, `iroh-blobs` for video payload and poster payload
- Replicated?: Yes
- Rebuildable From: `docs + blobs`
- Public Replica / Private Replica / Local Only: Public replica for `VideoManifest` / `VideoPoster` asset refs, local projection for blob status and preview cache
- Gossip Hint 必要有無: Yes, `TopicIndexUpdated` only
- Blob 必要有無: Yes
- SQLite projection 必要有無: Yes
- 必須 contract:
  - `video_post_visible_before_full_blob_download`
  - `iroh_transport_syncs_video_post_between_apps`
  - `late_joiner_backfills_video_post_from_docs`
  - `restart_restores_video_post_preview`
- 必須 scenario:
  - video composer UI 導入後に `video post -> poster skeleton -> poster preview` を回帰に追加する

## Decision
- 動画投稿は `VideoManifest` と `VideoPoster` の 2 種類の asset ref を header に載せる。
- 最小縦スライスでは `VideoManifest` は動画本体 blob を指す attachment として扱い、再生制御や adaptive manifest は後続フェーズに送る。
- `VideoPoster` は card renderer の preview source として扱い、blob 未取得時は poster skeleton を表示する。
- `gossip` は video bytes や poster bytes を運ばず、`TopicIndexUpdated` hint のみを publish する。
- `SQLite` は video post の timeline/thread projection、poster status、local preview cache に限定する。

## Consequences
- late joiner と restart 後の復元は `docs header + blobs fetch` だけで成立しなければならない。
- poster が未取得でも video post row 自体は timeline/thread に出なければならない。
- 最初の実装では再生 UI は入れず、poster preview までを完了条件とする。
