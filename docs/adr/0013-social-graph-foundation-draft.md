# ADR 0013: social-graph foundation draft

## Status
Draft

## Date
2026-03-19

## Base Branch
`main`

## Related
- `docs/adr/0011-kukuri-protocol-v1-draft.md`
- `docs/adr/0012-topic-first_progressive_community_filtering_draft.md`

## Feature Data Classification
- Feature 名: social-graph foundation
- Durable / Transient: Durable public graph state + local durable projection
- Canonical Source: public author docs replica (`author::<pubkey>`) に保存された author-signed profile / outgoing follow edge
- Replicated?: Yes, public author graph is replicated; derived relationship cache is local rebuildable
- Rebuildable From: author replicas + signed envelopes + local projection rebuild
- Public Replica / Private Replica / Local Only: public author replica for profile / follow edge, local only projection for mutual / friend-of-friend query cache
- Gossip Hint 必要有無: Yes, best-effort only
- Blob 必要有無: No
- SQLite projection 必要有無: Yes
- 必須 contract:
  - `author_replica_roundtrips_profile_and_follow_edge`
  - `social_graph_projection_derives_mutual_and_friend_of_friend_from_author_replicas`
  - `social_graph_restart_rebuilds_projection_from_author_replicas`
- 必須 scenario:
  - 2 desktop が相互 follow し、restart 後も mutual relationship が復元されること

## 1. Context

`docs/adr/0012-topic-first_progressive_community_filtering_draft.md` で確認したとおり、`friend-only` / `friend-plus` を current `main` に導入するには先に social-graph が必要である。

2026-03-19 時点の `main` には、social-graph の断片だけが存在する。

- `crates/core` に `Profile` と `identity-profile` parser がある
- `crates/core` に `GossipHint::ProfileUpdated` がある
- `crates/docs-sync` に `author_replica_id(author_pubkey)` がある

一方で、現時点では以下が未実装である。

- profile publish / hydrate の end-to-end flow
- directed follow edge
- mutual relationship 判定
- friend-of-friends 判定
- profile / relationship projection
- desktop の follow / unfollow 導線

したがって、本 ADR は social-graph を first-class feature として導入する最小範囲を定義する。

## 2. Problem

friend 系 audience を将来的に成立させるには、少なくとも次が必要になる。

- 誰が誰を follow しているかの canonical source
- mutual relationship と friend-of-friends を local に判定できる projection
- topic / community data plane とは独立した social sync plane

この基盤がないまま `friend-only` や `friend-plus` を UI label として先に出すと、

- 判定根拠が説明できない
- クライアントごとに結果がぶれる
- community-node に social truth を寄せる誘惑が生まれる

ので避ける。

## 3. Decision

social-graph v1 は、public author replica を canonical source にした author-owned graph として導入する。

### 3.1 v1 scope

v1 に含めるもの:

- public profile
- directed follow edge
- follow revoke
- local projection による `following` / `followed_by` / `mutual` 判定
- local projection による `friend_of_friend` 判定
- `friend_of_friend` の read-only 表示

v1 に含めないもの:

- friend-of-friends gating
- recommendation / ranking
- server-managed social graph
- private note / local alias / mute / block の同期
- community-node 上の follow registry

### 3.2 author replica を social graph の正本にする

各 author は public な author replica を持つ。

```text
author::<author_pubkey>
```

この replica には、その author が所有する public social objects を置く。

最低限:

- profile latest
- outgoing follow edges
- follow revoke tombstone

incoming edges は正本として持たない。`followed_by` や `mutual` は local projection で導出する。

### 3.3 authority は docs write 権ではなく署名で決める

current `main` の docs replica は deterministic secret で開けるため、replica への write capability そのものは権限境界にならない。

したがって social-graph v1 の authority は次で決める。

- profile は `envelope.pubkey == author_pubkey`
- follow edge は `subject_pubkey == envelope.pubkey`
- revoke も同一 author の署名でなければ無効

docs replica は配布・同期の媒体であり、trust anchor ではない。

## 4. Data Model

### 4.1 Envelope kinds

少なくとも次の signed envelope を導入する。

- `identity-profile`
- `follow-edge`

必要なら revoke は `follow-edge` の status 更新で表現してよい。

### 4.2 Author replica keys

正確な key 名は実装で確定してよいが、責務は次に固定する。

- `profile/latest`
- `graph/follows/<target_pubkey>`
- `envelopes/<envelope_id>`

`profile/latest` と `graph/follows/<target_pubkey>` は query 用の正規化 state、
`envelopes/<envelope_id>` は署名検証と監査用の原本保持を想定する。

### 4.3 Minimal document shapes

例:

```ts
type SocialProfileDocV1 = {
  author_pubkey: string
  name?: string
  display_name?: string
  about?: string
  picture?: string
  updated_at: number
  source_envelope_id: string
}

type FollowEdgeDocV1 = {
  subject_pubkey: string
  target_pubkey: string
  status: "active" | "revoked"
  updated_at: number
  source_envelope_id: string
}
```

### 4.4 Local projection

SQLite projection には少なくとも次を持つ。

- profile cache
- outgoing follow edge cache
- derived relationship cache

ここでいう `projection` は、canonical source ではなく local query のために再構築する cache / index を指す。author replica から再計算できるので、壊れても rebuild 可能である。

derived relationship cache は次を引ければよい。

- `following`
- `followed_by`
- `mutual`
- `friend_of_friend`

friend-of-friends は v1 から projection に入れ、read-only 表示にも使う。ただし topic audience や access control の gating にはまだ使わない。

## 5. Sync Model

### 5.1 topic sync とは別系統で扱う

social graph は topic timeline と同じ subscription map に押し込まない。

理由:

- topic は user-facing browsing unit
- social graph は author-centric state
- lifecycle と query pattern が異なる

したがって `app-api` には topic subscription とは別の author replica sync 管理が必要になる。

### 5.2 Sync 対象は無制限に広げない

v1 の sync 対象は次に制限する。

- local author replica
- 明示的に follow した author の replica
- active topic で観測した author の replica
- community / invite 導線で明示的に必要になった author

既知の全 pubkey を自動で追跡しない。friend-of-friends のために無制限に graph crawl すると、帯域と local cache が膨らみすぎる。

### 5.3 Hints

profile / social graph 変更の通知には hint を使ってよいが、truth source にはしない。

`ProfileUpdated` を流用するか、新しい `SocialGraphUpdated { author }` を足すかは実装時に選んでよい。ただし意味は「author replica に更新がある」で統一する。

## 6. Community-Node Boundary

community-node は social graph の canonical store にならない。

維持する境界:

- community-node は auth / consent / bootstrap / relay assist
- social graph の canonical source は docs + signed envelope
- mutual / friend-of-friends は local projection で計算する

community-node に follower list や friend graph を寄せると、`docs + blobs + hints` を正本とする current architecture と衝突する。

## 7. UX Boundary

social-graph v1 はすぐに friend-only UI を出すためのものではない。

ここでいう `product semantics` は、「内部で計算している」ではなく「UI や policy で実際に意味を持たせる振る舞い」を指す。たとえば `friend_of_friend` を badge として表示するのは product semantics に含まれるが、その結果で access control を変えるのはさらに強い semantics であり、別段階として扱う。

first ship の UX は次に留める。

- profile 編集
- follow / unfollow
- relationship 表示 (`following`, `follows you`, `mutual`)
- `friend of friend` 表示

まだ出さないもの:

- `friend-only` audience selector
- `friend-plus` recommendation
- social graph ベースの topic ranking

これにより、graph foundation と audience semantics を段階的に切り離せる。

## 8. Required Implementation Work

### 8.1 `crates/core`

- social profile / follow edge の envelope content
- signed envelope builder / parser
- relationship status enum

### 8.2 `crates/docs-sync`

- author replica utility の正式利用
- author replica query / subscribe の contract 整備

private secret 管理は不要。social-graph v1 は public graph だからである。

### 8.3 `crates/app-api`

- profile publish / load
- follow / unfollow command
- author replica hydration
- projection 更新
- mutual relationship query

### 8.4 `crates/store`

- profile cache table
- follow edge table
- relationship derivation query

### 8.5 `apps/desktop`

- profile editor
- author card
- follow / unfollow action
- relationship badge

## 9. Rollout

### Phase 1

profile foundation

- local profile publish
- remote profile hydrate
- author replica sync

### Phase 2

directed follow graph

- follow / unfollow
- local projection
- mutual derivation
- friend-of-friends derivation
- read-only `friend of friend` 表示

### Phase 3

audience integration prerequisites

- private channel / membership design と接続
- friend-only を mutual follow の上に再定義できる状態にする
- friend-plus gating は別 ADR で有効化条件を再審査する

## 10. Consequences

- `friend-only` / `friend-plus` は social-graph v1 完了前に ship しない
- social graph は public by default で導入する
- private contacts / blocks / notes は別 feature として扱う
- community-node は social graph の server truth にならない

## 11. Open Questions

- `ProfileUpdated` を流用するか、`SocialGraphUpdated` を追加するか
- relationship cache を materialized に持つか、query で都度導出するか
- local-only mute / block / alias を同じ ADR で扱うか、別 ADR に分離するか

## 12. Decision Summary

social-graph 導入は、friend 系 audience を先に実装するためではなく、まず public author-owned graph を成立させるために行う。

current `main` に対する最小で妥当な導入順は次である。

1. author replica に public profile と outgoing follow edge を載せる
2. local projection で `mutual` と `friend_of_friend` を導出する
3. `friend_of_friend` は read-only 表示として先行導入する
4. その上で `friend-only` / `friend-plus` gating を別段階で再定義する

この順序なら、topic-first / docs-first / community-node control plane という既存境界を崩さずに social-graph を追加できる。
