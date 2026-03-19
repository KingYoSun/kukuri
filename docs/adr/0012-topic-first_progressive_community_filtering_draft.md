# ADR 0012: topic-first 段階的コミュニティ絞り込みドラフト

## Status
Draft

## Date
2026-03-19

## Base Branch
`main`

## Related
- `docs/adr/0008-dht-discovery-data-classification.md`
- `docs/adr/0009-community-node-relay-auth-data-classification.md`
- `docs/adr/0010-kukuri-protocol-v1-boundary-definition.md`
- `docs/adr/0011-kukuri-protocol-v1-draft.md`
- `docs/adr/0013-social-graph-foundation-draft.md`

## Feature Data Classification

| 対象 | Canonical Source | Local Cache / Projection | 2026-03-19 時点 | 次フェーズでの扱い |
|---|---|---|---|---|
| public post / thread / live / game | `public docs replica + blobs + topic hint` | SQLite projection | 実装済み | 現状維持 |
| invite-only private channel data | `private docs replica + blobs + private hint` | SQLite projection | Phase 1 実装済み | hard-private baseline として継続利用 |
| joined private channel capability | local secure capability storage | なし | Phase 1 実装済み | `friend_only` / `friend_plus` でも同期 gate に使う |
| social graph profile / follow edge | public author docs replica (`author::<pubkey>`) | SQLite projection | `0013` 実装済み | friend 系 audience の relationship 判定入力に使う |
| derived relationship (`mutual`, `friend_of_friend`) | author replica + signed follow edge からの local projection | rebuildable local cache | `0013` 実装済み | `friend_only` 判定と `friend_plus` sponsor 検証に使う |
| channel audience policy | channel metadata + owner-signed policy doc | SQLite projection | `invite_only` 相当のみ実装済み | `friend_only` / `friend_plus` policy を追加する |
| join grant / epoch control | current epoch capability + share token / grant metadata | local secure capability storage | `invite_only` の owner-origin grant のみ実装済み | `friend_only` は owner-controlled, `friend_plus` は participant-mediated chain を追加する |
| community-node auth / relay assist | community-node config + token cache | local file / secure storage | 実装済み | control plane のまま据え置く |

## 1. Context

旧版の `0012` は、`friend_only` / `friend_plus` を検討する前提として social graph が未実装であり、Phase 1 の `invite_only` も未実装だった時点の文脈を引きずっていた。

しかし 2026-03-19 時点の `main` はすでに次の状態にある。

- Phase 1 の invite-only private channel は実装済み
- `docs/adr/0013-social-graph-foundation-draft.md` の social graph v1 も実装済み
- desktop には `Create Channel`, `Create Invite`, `Join via Invite`, `View Scope`, `Compose Target` が入っている
- app/runtime/docs-sync/harness/frontend に private channel と social graph の contract がある

加えて `friend_plus` は、既存プロダクト文脈では owner の `friend_of_friend` snapshot ではなく、「参加者の mutual を順次たどって join 可能になる mutual の連鎖」として理解されやすい。この期待から外れると UX 上の誤解を招く。

また、プロダクト上の `friend_plus` は hard-private ではなく `public` と `private` の中間に置く audience として意図されている。したがって `friend_plus` は、`invite_only` や `friend_only` と同じ漏洩耐性を目標にしない。

そのため `0012` は、次の 2 系統を明示する文書へ更新する。

- hard-boundary audience: `invite_only`, `friend_only`
- soft-boundary audience: `friend_plus`

## 2. Current Main Snapshot

### 2.1 Phase 1 invite-only は出荷済み

現行実装の private channel baseline はすでに成立している。

- `crates/docs-sync` は `channel::<channel_id>` replica を public deterministic secret から除外し、registered capability がなければ open できない
- `crates/app-api` は `create_private_channel`, `export_private_channel_invite`, `import_private_channel_invite`, `list_joined_private_channels` を持つ
- `crates/desktop-runtime` は private channel capability を local secure storage に保存し、restart 後に復元する
- `apps/desktop` は topic-first UI のまま `View Scope` / `Compose Target` / `Create Channel` / `Create Invite` / `Join via Invite` を提供する

Phase 1 の回帰は少なくとも次で固定されている。

- `private_replica_requires_registered_capability`
- `private_channel_invite_scopes_posts_and_replies`
- `private_channel_invite_restores_after_restart_without_reimport`
- `private_channel_invite_connectivity`

### 2.2 social graph v1 も出荷済み

`0013` で定義した social graph foundation も、もはや前提条件ではなく current baseline である。

- canonical source は public author replica (`author::<pubkey>`)
- follow / unfollow は signed follow edge で表現される
- local projection から `following`, `followed_by`, `mutual`, `friend_of_friend` を導出する
- desktop は profile 表示、follow / unfollow、relationship badge、`friend of friend` 表示を持つ

回帰としては少なくとも次が存在する。

- `social_graph_derives_friend_of_friend_and_clears_after_unfollow`
- store migration / projection tests for follow edge and relationship cache
- desktop follow/unfollow / relationship badge tests

### 2.3 未実装なのは audience semantics 側

まだ入っていないのは social graph そのものではなく、その graph を audience policy に接続する層である。

- `friend_only` の strict policy
- `friend_plus` の mutual-chain policy
- sponsor / join reason を伴う capability share
- hard-boundary revoke と soft-boundary frontier stop の違い
- friend 系 audience を UI で説明するための理由表示

## 3. Problem Reframed

`friend_only` と `friend_plus` を同じ privacy model で扱うと、どちらかが壊れる。

- `friend_only` は owner 起点で閉じた hard-private audience として設計すべき
- `friend_plus` は参加者から参加者へ広がる soft-private audience として設計すべき

両者とも topic-first / channel-first の枠内に置けるが、capability の配り方と revoke 期待値は分けて考える必要がある。

整理すると、今後の audience は次の 3 つになる。

1. `invite_only`: explicit invite による hard-private
2. `friend_only`: owner-scoped mutual による hard-private
3. `friend_plus`: participant-scoped mutual chain による soft-private

## 4. Decision

### 4.1 topic-first は維持し、channel を audience 単位とする

topic-first の UX と sync model は維持する。

- topic は discovery / browsing / timeline の軸
- channel は audience / capability / hint scope の軸

`friend_only` / `friend_plus` を導入しても、topic を social graph 単位へ置き換えない。追加するのは topic 配下の channel policy だけである。

### 4.2 Phase 1 invite-only を private audience baseline とする

`invite_only` は future plan ではなく、以後の audience feature が依存する baseline である。

今後の `friend_only` / `friend_plus` も、Phase 1 で入った次の構成を再利用する。

- private replica
- private hint topic
- secure capability persistence
- restart restore
- topic-first UI 上の scope 切り替え

### 4.3 `friend_only` は owner-scoped `mutual` を policy 入力にする

`friend_only` は hard-private audience として定義する。

意味論:

- owner と recipient が相互 follow なら candidate member
- one-way follow は含めない
- 判定根拠は `0013` の author replica + local relationship projection
- join grant は owner 起点または owner の current device 起点で行う

ここでの `friend` は channel owner 視点の `mutual` を指す。参加者の連鎖で membership を広げない。

### 4.4 `friend_plus` は participant-scoped `mutual` の連鎖を policy 入力にする

`friend_plus` は hard-private ではなく soft-private audience として定義する。

意味論:

- current participant は、自分と `mutual` な相手を join させられる
- join した participant は新しい sponsor になり、自分の `mutual` をさらに join させられる
- この挙動は current epoch の間、参加者の連鎖として伝播してよい
- one-way follow は含めない
- owner と直接つながっていない participant でも、途中の participant との `mutual` が成立していれば join できる

つまり `friend_plus` の定義は owner の `mutual ∪ friend_of_friend` snapshot ではない。正本は「現在の参加者集合から辿れる pairwise mutual の連鎖」である。

`0013` の `friend_of_friend` projection は、UI 上の説明や join suggestion には使ってよいが、`friend_plus` の意味そのものにはしない。

### 4.5 enforcement は audience 種別ごとに分ける

`invite_only` と `friend_only` は hard-boundary として扱う。

- private replica capability が同期 gate になる
- owner-controlled な grant / invite を使う
- revoke や relationship downgrade では epoch rotation を前提にする
- rotation 後は旧 capability で新 epoch を open できないことを目標にする

一方 `friend_plus` は soft-boundary として扱う。

- private replica capability は引き続き同期 gate に使う
- ただし capability の配布は owner 限定にしない
- current participant が current epoch の share token を mutual に渡せる
- 受信側は sponsor との `mutual` を満たす限り join できる
- join 後は自分も sponsor になれる

### 4.6 `friend_plus` の漏洩境界を明示する

`friend_plus` は public と hard-private の中間であり、漏洩リスクを一定範囲で受け入れる。

この ADR で許容するのは次までである。

- current participant が、自分の `mutual` に current epoch を伝播できる
- owner の直視範囲を超えて participant frontier が広がる
- relationship 断絶や退出が起きても、epoch rotation までの間は current epoch が残りうる
- 一度受信済みの content を完全に回収できない

この ADR でまだ許容しないのは次である。

- `mutual` でない相手への join
- sponsor 不明の匿名 join
- public listing や検索経由での discoverability
- community-node を truth source にした membership 拡張
- owner rotation 後も無期限に使える non-expiring grant

実装時にこの漏洩境界を広げる変更が必要になった場合は、都度あらためて確認する。

### 4.7 community-node の責務は広げない

community-node は引き続き control plane に限定する。

- auth / consent
- relay assist
- bootstrap node / seed peer registration

やらないこと:

- social graph の truth source 化
- friend-only / friend-plus membership registry
- private channel invite / grant registry

friend 系 audience の判定も capability 配布も、docs-first の境界内で完結させる。

## 5. Proposed Model For Future Phases

### 5.1 channel policy を第一級概念にする

少なくとも次の policy kind を持つ。

```text
public
invite_only
friend_only
friend_plus
```

`public` は open、`invite_only` と `friend_only` は hard-private、`friend_plus` は soft-private として扱う。

### 5.2 hard-boundary path: `invite_only` / `friend_only`

この 2 つは owner-controlled grant model を維持する。

- owner が join 権限を決める
- `friend_only` は owner-scoped `mutual` を使って candidate 判定する
- actual sync は epoch capability で gate する
- revoke / downgrade は epoch rotation を伴う

これは強い secrecy が必要な lane である。

### 5.3 soft-boundary path: `friend_plus`

`friend_plus` は participant-mediated join model を採る。

- current participant が sponsor になる
- sponsor は自分と `mutual` な相手に share token を渡せる
- token は current epoch capability か、その縮約表現を運ぶ
- recipient は sponsor との `mutual` を確認できたら join する
- join 後は participant frontier に入り、自分も sponsor になれる

このモデルでは、participant frontier が social graph 上で連鎖的に広がる。

### 5.4 `friend_plus` に必要な追加状態

少なくとも次が必要になる。

- sponsor を識別できる join metadata
- `joined via <author>` を表示するための reason data
- participant frontier を local で追跡する state
- owner が future expansion を止めるための freeze / rotate 操作

`friend_of_friend` projection は suggestion や diagnostics には使ってよいが、frontier truth の代替にはしない。

### 5.5 grant carrier は follow-up で確定する

grant / share の carrier は follow-up ADR で確定してよい。候補は次のようなものがある。

- direct share token
- owner / participant authored docs object
- device-targeted mailbox 的な docs path

ただし前提は固定する。

- community-node へは置かない
- server-managed ACL にはしない
- sponsor または owner を説明可能にする

## 6. Rollout

### Phase 1

invite-only private channel baseline

状態:

- 実装済み
- `Create Channel -> Create Invite -> Join via Invite`
- post / reply-thread / live / game
- restart 復元
- fresh invite 再発行

### Phase 2

`friend_only` policy integration

必要なもの:

- channel policy doc に `friend_only` を追加
- owner-scoped `mutual` から candidate member を導出
- owner-controlled grant
- relationship downgrade 時の epoch rotation
- diagnostics / UI で `Friends` policy を説明可能にする

### Phase 3

`friend_plus` policy integration

必要なもの:

- channel policy doc に `friend_plus` を追加
- participant-sponsored share flow
- pairwise mutual 検証による chain join
- `joined via <author>` の reason 表示
- owner による freeze / rotate
- candidate expansion に対する abuse / spam guardrail

### Out of Scope

この ADR ではまだ入れないもの:

- explicit member list moderation
- approval workflow
- admin / moderator role
- block / mute / local alias
- community-node managed audience

これらは separate ADR で扱う。

## 7. Test Strategy

既存の Phase 1 regressions は維持したうえで、次フェーズでは少なくとも次を required にする。

- store / app-api: `friend_only` は owner-scoped mutual を失ったら epoch rotate が必要になる
- app-api / runtime: `friend_plus` で B が owner 経由で join した後、C は B と `mutual` なら join でき、owner と直接関係がなくても許可される
- app-api / runtime: D が current participant の誰とも `mutual` でなければ `friend_plus` に join できない
- docs-sync / runtime: `friend_only` は revoked peer が旧 capability のまま新 epoch を open できない
- runtime / desktop: `friend_plus` では sponsor reason (`joined via ...`) を表示できる
- harness: 4 client 以上で `owner -> friend join -> chained friend join -> freeze/rotate -> further join blocked` を自動確認する

invite-only Phase 1 の既存回帰も引き続き required とする。

## 8. Consequences

- `invite_only` は shipping baseline になる
- `friend_only` は hard-private として設計する
- `friend_plus` は soft-private として設計する
- `friend_plus` では participant mutual chain に沿った漏洩リスクを受け入れる
- ただしその漏洩境界を超える変更は、実装時に別途確認が必要になる
- community-node は引き続き membership truth にならない

## 9. Decision Summary

2026-03-19 時点の `main` を前提にした更新後の判断は次のとおり。

- Phase 1 の `invite_only` はすでに実装済み
- `0013` の social graph foundation もすでに実装済み
- `friend_only` は owner-scoped `mutual` による hard-private として再設計する
- `friend_plus` は owner snapshot ではなく、participant-scoped `mutual` の連鎖による soft-private として再設計する
- access control の扱いは audience ごとに分け、`friend_plus` では中間 audience としての漏洩許容を明示する

この方針なら、topic-first / docs-first / community-node control plane という現行境界を崩さずに、`friend_plus` を既存ユーザー期待とプロダクト意図に合わせて再定義できる。
