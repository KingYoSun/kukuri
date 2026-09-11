# ADR 0022: Local Author Mute And Social Management

## Status
Proposed

## Date
2026-03-31

## Base Branch
`main`

## Related
- `docs/adr/0002-feature-data-classification-template.md`
- `docs/adr/0013-social-graph-foundation-draft.md`
- `docs/adr/0018-channel-first-sidebar-and-unified-epoch-lifecycle.md`
- `docs/adr/0021-local-post-bookmark-data-classification.md`
- `docs/adr/0043-dome-access-presence-spatial-audio.md`（signed block edge の正本。#961 で block を content surface の非表示にも使う）

## Feature Data Classification
- Feature 名: local author mute + profile social management
- Durable / Transient: durable local-only mute state + local route/UI state
- Canonical Source: local SQLite `muted_authors`
- Replicated?: No
- Rebuildable From: local mute record, follow edges, local profile cache
- Public Replica / Private Replica / Local Only:
  - mute collection: local only
  - following / followed projection: 既存 author replica + local projection
- Gossip Hint 必要有無: No
- Blob 必要有無: No
- SQLite projection 必要有無: Yes
- 必須 contract:
  - `mute_author_restores_after_restart`
  - `muted_author_is_filtered_from_timeline_thread_profile_and_bookmarks`
  - `muted_author_is_filtered_from_live_and_game_lists`
  - `repost_of_muted_author_is_hidden`
  - `mute_does_not_change_follow_mutual_or_friend_gating`
  - `list_social_connections_followed_is_local_known_only`
  - `unmute_restores_visibility`
  - `blocking_author_hides_posts_until_unblocked`（#961）
  - `blocked_by_author_hides_their_posts_until_revoked_but_mute_persists`（#961）
  - `repost_of_blocked_author_is_hidden`（#961）
  - `blocked_author_is_filtered_from_live_and_game_lists_in_both_directions`（#961）
- 必須 scenario:
  - `follow -> local mute on one device -> muted device only hides post/live/game -> restart restore`
  - 別 device では mute state が共有されないこと

## Decision
- author mute v1 は特定 author の `post / live / game` を current device だけで非表示にする local visibility preference とする。
- mute の canonical source は `ProjectionStore` の `muted_authors` で、docs sync、gossip、community-node、他 device には複製しない。
- mute は social graph 本体には入れず、follow edge、mutual、friend-only / friend-plus gating、DM 可否、private channel access を変更しない。
- ADR-0043のsigned blockとは独立する。Dome音声ではmuteはlocal playbackだけを抑止し、blockだけがConnection/accessを失効させる。
- Following / Followed / Muted / Blocked の管理 UI は新しい primary section を増やさず、`#/profile` 配下の `profileMode=connections` として持つ。設定 > セーフティはミュートとブロックの違いを説明し、この画面へ遷移する導線だけを持ち、一覧を複製しない（#961）。
- `connectionsView` は `following | followed | muted | blocking` を持ち、invalid 値は `following` に normalize する。`blocked_by`（自分をブロックしている相手）の一覧やバッジは UI に出さない（#961）。
- `Followed` 一覧は full follower inventory ではなく、この端末で hydrate 済みの incoming follow だけを表示する local-known list とする。
- mute 対象 author は `list_timeline`, `list_thread`, `list_profile_timeline`, `list_bookmarked_posts`, `list_live_sessions_scoped`, `list_game_rooms_scoped` から除外する。
- #961: 同じ 6 経路は、ADR-0043 の signed block edge が Active な相手も除外する。向きは問わず、自分がブロックした相手と、自分をブロックした相手（その edge をこの端末が hydrate した時点）の両方を隠す。これは表示上の非表示であり、投稿・replica の取得や保存、通知、DM、follow、Dome の access 判定は変えない。edge が Revoked になれば再表示し、mute 中なら隠れたままにする。非表示集合の helper は `current_hidden_author_pubkeys`（muted ∪ blocking ∪ blocked_by）。
- repost / quote repost は card author が未ミュートでも `repost_of.source_author_pubkey` が muted なら非表示にする。
- author detail と profile social management page では muted / blocked author を表示し、unmute / unblock 導線を失わないようにする。一覧の各行と作者詳細、通報ダイアログのローカル操作から block / unblock できる（#961）。
- Following / Followed / Muted の一覧は `display_name -> name -> pubkey` の昇順に固定する。

## Public Interfaces
- `kukuri-store`
  - `MutedAuthorRow`
  - `put_muted_author`
  - `get_muted_author`
  - `list_muted_authors`
  - `remove_muted_author`
- `kukuri-app-api`
  - `AuthorSocialView.muted`
  - `SocialConnectionKind`
  - `mute_author(pubkey)`
  - `unmute_author(pubkey)`
  - `list_social_connections(kind)`
- `desktop-runtime` / Tauri / `apps/desktop/src/lib/api.ts`
  - mute/list social connection APIs をそのまま公開する
- desktop shell route
  - `#/profile?topic=<topic>&profileMode=connections&connectionsView=following|followed|muted|blocking`

## Consequences
- mute は local-only preference なので cross-device sync されず、端末ごとに異なる visibility が成立する。
- content surface では muted author を完全に隠す一方、management surface では表示するため、解除導線は維持される。
- block による非表示は相手の replica を hydrate して初めて効くため、相手側で反映されるまでに遅延がある。厳密な取得禁止ではない（#961 で妥協として固定）。
- `Followed` は observed-only list なので UI に不完全性の説明が必要になる。
- bookmark 一覧も mute filter の対象に入るため、保存済み post であっても muted author なら通常の content surface からは見えなくなる。
