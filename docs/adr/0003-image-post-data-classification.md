# ADR 0003: Image Post Data Classification

## Status
Accepted

## Feature Data Classification
- Feature 名: image post
- Durable / Transient: Durable
- Canonical Source: `iroh-docs` for post header / topic index, `iroh-blobs` for image payload
- Replicated?: Yes
- Rebuildable From: `docs + blobs`
- Public Replica / Private Replica / Local Only: Public replica for post header and asset refs, local projection for blob status cache
- Gossip Hint 必要有無: Yes, `TopicIndexUpdated` only
- Blob 必要有無: Yes
- SQLite projection 必要有無: Yes
- 必須 contract:
  - `gossip_hint_contains_no_payload_body`
  - `image_post_visible_before_full_blob_download`
  - `gossip_loss_does_not_lose_durable_post`
- 必須 scenario:
  - image composer UI 導入後に `image post -> placeholder visible -> blob fetch -> visible` を追加する

## Decision
- 画像投稿の shared durable metadata は `CanonicalPostHeader.attachments` を通じて `docs` に保存する。
- 画像本体は `blobs` に保存し、`AssetRef` の `hash / mime / bytes / role` だけを header に載せる。
- `gossip` は `TopicIndexUpdated` hint のみを運び、画像 bytes や本文 bytes は運ばない。
- `SQLite` は image post の timeline/thread projection と blob status cache に限定する。

## Consequences
- late joiner は `docs` の header と `blobs` の asset fetch だけで画像投稿を復元できる。
- timeline は blob 未取得でも post row と attachment metadata を表示できなければならない。
- 画像 composer UI を入れる前でも、contract では attachment metadata と blob status 遷移を保証する。
