# ADR 0019: profile avatar blob data classification

## Status
Accepted

## Date
2026-03-29

## Base Branch
`main`

## Related
- `docs/adr/0013-social-graph-foundation-draft.md`
- `docs/adr/0010-kukuri-protocol-v1-boundary-definition.md`

## Feature Data Classification
- Feature 名: profile avatar blob
- Durable / Transient: Durable public profile metadata + durable blob payload
- Canonical Source: public author docs replica (`author::<pubkey>`) for profile header, `iroh-blobs` for avatar image payload
- Replicated?: Yes, profile doc is replicated and avatar blob is fetchable from blob sync
- Rebuildable From: author replica profile doc + signed profile envelope + avatar blob
- Public Replica / Private Replica / Local Only: public author replica for profile metadata, local preview state only for pre-publish file selection
- Gossip Hint 必要有無: Yes, best-effort only
- Blob 必要有無: Yes
- SQLite projection 必要有無: Yes
- 必須 contract:
  - `profile_envelope_roundtrip`
  - `store_profile_upsert_latest_wins`
  - `set_my_profile_persists_blob_backed_avatar_and_restores_it`
  - `desktop runtime restart restores blob backed avatar`
- 必須 scenario:
  - 実機 2 台で `profile image select -> save -> remote author detail reflect -> restart restore` を確認する

## Decision

profile avatar は URL 文字列を primary source にしない。current `main` では、public author replica に保存された author-signed profile envelope/doc を header とし、画像本体は blob として同期する。

legacy `picture` URL は read compatibility のため維持してよいが、新規 desktop UI の primary write path は blob-backed avatar に固定する。

## Data Model

- signed envelope kind は既存の `identity-profile` を継続利用する
- `KukuriProfileEnvelopeContentV1` と `AuthorProfileDocV1` は optional `picture_asset` を持つ
- `picture_asset` は `hash / mime / bytes / role=profile_avatar` を持つ
- `Profile` / `AuthorSocialView` は `picture_asset` を public contract に含める
- SQLite `profiles` / `profile_cache` は `picture_blob_hash / picture_mime / picture_bytes` を保持する

## Write Path

1. desktop UI は `image/*` file picker で avatar file を受け取る
2. runtime/app-api は file bytes を `iroh-blobs` へ put する
3. resulting blob ref を `picture_asset` として `identity-profile` envelope に含める
4. author replica の `profile/latest` doc と local projection cache を更新する

## Read Path

1. profile / author detail は `picture_asset` があればそれを優先する
2. client は既存の blob fetch path で avatar blob を lazy fetch する
3. `picture_asset` がない場合だけ legacy `picture` URL を fallback として使う

## Non-Goals

- avatar crop editor
- private avatar
- animated avatar 専用 manifest
- community-node hosted profile image canonicalization
