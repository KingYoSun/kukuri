# ADR 0012: topic-first 段階的コミュニティ絞り込み

## Status
Accepted

## Date
2026-03-20

## Base Branch
`main`

## Related
- `docs/adr/0008-dht-discovery-data-classification.md`
- `docs/adr/0009-community-node-relay-auth-data-classification.md`
- `docs/adr/0010-kukuri-protocol-v1-boundary-definition.md`
- `docs/adr/0011-kukuri-protocol-v1-draft.md`
- `docs/adr/0013-social-graph-foundation-draft.md`

## Feature Data Classification

| 対象 | Canonical Source | Local Cache / Projection | 2026-03-20 時点 | 次フェーズでの扱い |
|---|---|---|---|---|
| public post / thread / live / game | `public docs replica + blobs + topic hint` | SQLite projection | 実装済み | 現状維持 |
| invite-only private channel data | `private docs replica + blobs + private hint` | SQLite projection | Phase 1 実装済み | hard-private baseline として継続利用 |
| joined private channel capability | local secure capability storage | なし | Phase 1 実装済み | `friend_only` / `friend_plus` でも同期 gate に使う |
| social graph profile / follow edge | public author docs replica (`author::<pubkey>`) | SQLite projection | `0013` 実装済み | friend 系 audience の relationship 判定入力に使う |
| derived relationship (`mutual`, `friend_of_friend`) | author replica + signed follow edge からの local projection | rebuildable local cache | `0013` 実装済み | `friend_only` 判定と `friend_plus` sponsor 検証に使う |
| channel audience policy | channel metadata + owner-signed policy doc | SQLite projection | `invite_only` / `friend_only` / `friend_plus` 実装済み | member moderation / approval は separate ADR で扱う |
| join grant / epoch control | current epoch capability + signed share token + encrypted rotation grant | local secure capability storage | owner-origin invite、friend-only grant、friend-plus share / rotation grant 実装済み | candidate expansion の guardrail は separate task で扱う |
| community-node auth / relay assist | community-node config + token cache | local file / secure storage | 実装済み | control plane のまま据え置く |

## 1. Context

旧版の `0012` は、`friend_only` / `friend_plus` を検討する前提として social graph が未実装であり、Phase 1 の `invite_only` も未実装だった時点の文脈を引きずっていた。

しかし 2026-03-20 時点の `main` はすでに次の状態にある。

- Phase 1 の invite-only private channel は実装済み
- `docs/adr/0013-social-graph-foundation-draft.md` の social graph v1 も accepted baseline として実装済み
- `friend_only` / `friend_plus` audience と `Create Grant` / `Create Share` / `Freeze` / `Rotate` も current `main` に入っている
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

`0013` で固定した social graph foundation も、もはや前提条件ではなく current baseline である。

- canonical source は public author replica (`author::<pubkey>`)
- follow / unfollow は signed follow edge で表現される
- local projection から `following`, `followed_by`, `mutual`, `friend_of_friend` を導出する
- desktop は profile 表示、follow / unfollow、relationship badge、`friend of friend` 表示を持つ

回帰としては少なくとも次が存在する。

- `store_profile_upsert_latest_wins`
- `author_relationship_projection_rebuild_roundtrip`
- `social_graph_derives_friend_of_friend_and_clears_after_unfollow`
- `post card shows friend of friend badge and author name fallback`
- `author detail shows via authors and follow action updates relationship`
- `local profile editor saves profile draft`

### 2.3 残っているのは guardrail / moderation 側

social graph と audience policy の接続そのものは current `main` に入っている。残っているのは hardening と周辺運用ルールである。

- `friend_plus` の candidate expansion に対する abuse / spam guardrail
- explicit member list / approval workflow / moderator role
- friend 系 audience の理由表示や explainability の追加磨き込み

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

## 5. Implementation Contract

### 5.1 logical channel と epoch を分離する

`friend_plus` は logical channel を維持したまま、同期 secret だけを epoch 単位で切り替える。

- `channel_id` は user-facing な stable ID とする
- `epoch_id` は replica secret の切り替え単位とする
- 新規 private replica は `channel::<channel_id>::epoch::<epoch_id>` を使う
- 既存 invite-only channel は migration 上 `legacy` epoch として読めるようにする

logical channel の timeline / thread / live / game は、local が保持している同一 `channel_id` の全 epoch を束ねて読む。新規 write は current epoch のみへ流す。

### 5.2 channel policy と participant state を signed object にする

`friend_plus` 実装で正本にする signed object は次で固定する。

- `channel-policy`
  - owner 署名
  - `audience_kind`, `epoch_id`, `sharing_state(open|frozen)`, `owner_pubkey`, `rotated_at`, `previous_epoch_id?`
- `channel-participant`
  - participant 署名
  - `participant_pubkey`, `epoch_id`, `joined_at`, `join_mode`, `sponsor_pubkey?`, `share_token_id?`
- `channel-share`
  - sponsor 署名の direct token
  - `channel_id`, `topic_id`, `channel_label`, `owner_pubkey`, `epoch_id`, `namespace_secret_hex`, `expires_at`
- `channel-rotation-grant`
  - owner 署名の encrypted grant
  - `channel_id`, `new_epoch_id`, `new_namespace_secret_hex`

`friend_of_friend` projection は UI / diagnostics 用に使ってよいが、join 可否の正本にはしない。

### 5.3 `friend_plus` share / import の契約

carrier は v1 では direct signed token で固定する。

share:

- current epoch の active participant なら誰でも sponsor になれる
- sponsor は自分と `mutual` な相手へ multi-use token を渡せる
- `expires_at` 未指定時の既定値は `now + 24h`
- `freeze` または `rotate` 済み epoch の token は import で reject する

import:

- token の署名と expiry を先に検証する
- sponsor author replica を warm し、social projection を更新する
- import 時点で `mutual(sponsor, local_author)` を必須にする
- token の epoch replica を暫定 open する
- replica 内の `channel-policy` から `audience_kind=friend_plus`, `sharing_state=open`, `token.epoch_id == current epoch` を確認する
- `channel-participant` から sponsor が current epoch の active participant であることを確認する
- recipient は immediate sponsor metadata 付きで自分の `channel-participant` を書く
- capability を current epoch として保存し、必要なら旧 epoch は archived へ落とす

### 5.4 `freeze` / `rotate` の契約

`freeze` と `rotate` は別操作だが、`rotate` は old current epoch を先に freeze してから進む。

`freeze`:

- owner-only
- old current epoch の `channel-policy.sharing_state` を `frozen` へ更新する
- その epoch での新規 share export を禁止する
- その epoch の未使用 token import を reject する

`rotate`:

- owner-only
- old current epoch を `frozen` にする
- 新しい `epoch_id` と `namespace_secret_hex` を生成する
- new epoch replica に metadata と `channel-policy(open)` を書く
- rotate 時点の active participant を new epoch の participant として複製する
- old frozen epoch に participant ごとの encrypted `channel-rotation-grant` を書く
- participant は old epoch を監視して自分向け grant を復号し、新 epoch を current として保存する
- old epoch は archived history として保持する

`rotate` は participant 集合を維持し、sharing frontier だけをリセットする。new epoch は `open` で始める。

### 5.5 rotation grant の暗号化方式

rotation grant は author pubkey 単位で暗号化する。

- `secp256k1` の `ecdh` を使う
- author key から BIP340 互換の shared secret を導出する
- HKDF-SHA256 で content key を導出する
- XChaCha20-Poly1305 で grant payload を暗号化する
- same-author multi-device でも decrypt できるよう、recipient は device ではなく author pubkey とする

old epoch に new epoch secret を平文で置く設計は stale token へ漏れるため採用しない。

### 5.6 Desktop / API 契約

API / UI では次を追加・拡張する。

- channel 作成 API は `audience_kind` を受け付ける
  - 既存 `create_private_channel` は未指定時 `invite_only` を既定にして後方互換を保つ
- invite-only API は維持し、`friend_plus` 用に以下を追加する
  - `export_friend_plus_share`
  - `import_friend_plus_share`
  - `freeze_private_channel`
  - `rotate_private_channel`
- joined channel view / capability には少なくとも以下を持たせる
  - `audience_kind`
  - `sharing_state`
  - `joined_via_pubkey`
  - `current_epoch_id`
  - `current_epoch_secret`
  - `archived_epoch_capabilities`
  - `creator_pubkey` / `owner_pubkey`
  - `is_owner`

UI:

- `friend_plus` channel では `Share`, `Freeze`, `Rotate` を表示する
- `Share` は current epoch の active participant にだけ表示する
- `Freeze` / `Rotate` は owner にだけ表示する
- channel detail に `joined via <short pubkey>` を表示する
- `invite_only` の既存 UI は維持する

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

状態:

- 実装済み
- channel policy doc に `friend_only` を追加
- owner-scoped `mutual` から candidate member を導出
- owner-controlled grant
- relationship downgrade 時に `rotation_required` を立て、owner rotate を要求する
- diagnostics / UI で `Friends` policy を説明できる

### Phase 3

`friend_plus` policy integration

状態:

- 実装済み
- epoch-aware channel capability
- direct signed `channel-share`
- import 時点の pairwise mutual 検証
- immediate sponsor metadata と `joined via ...` 表示
- owner-only `freeze`
- encrypted `channel-rotation-grant` による `rotate`
- archived/current epoch を束ねる logical channel read path
- rotate 後 newcomer は新 epoch content のみ読める
- candidate expansion に対する abuse / spam guardrail は未着手

### Out of Scope

この ADR ではまだ入れないもの:

- explicit member list moderation
- approval workflow
- admin / moderator role
- block / mute / local alias
- community-node managed audience
- one-time-use share registry
- full sponsor chain persistence
- epoch 間 history copy

これらは separate ADR で扱う。

## 7. Test Strategy

既存の Phase 1 regressions は維持したうえで、次フェーズでは少なくとも次を required にする。

Core:

- `channel-share` の roundtrip
- expiry reject
- 署名者不一致 reject
- `channel-rotation-grant` の encrypt/decrypt roundtrip
- 誤 recipient での decrypt failure

App / API:

- owner -> B join -> B -> C share で、`mutual(B,C)=true` なら C が join できる
- D が sponsor と `mutual` でなければ forwarded token では join できない
- `freeze` 後に old token import が失敗する
- `rotate` 後に old epoch token では new epoch secret を取得できない
- sponsor は import 時点で current epoch participant でなければならない
- logical channel scope が archived/current epoch を束ねて読める
- rotate 後の newcomer は new epoch の content だけ見える

Runtime:

- 既存 invite-only capability が epoch-aware 形式へ migrate される
- encrypted rotation grant を restart 後にも redeem できる
- current / archived epoch state が restart 後も復元される

Frontend / Harness:

- `friend_plus` の control が owner / participant 状態に応じて正しく出る
- `joined via ...` が表示される
- `freeze` 後は old epoch の share が無効になる
- 4 client scenario: owner -> B join -> B -> C join -> freeze で stale token block -> rotate -> B/C が grant で新 epoch へ移行 -> D の old token は失敗

invite-only Phase 1 の既存回帰も引き続き required とする。

## 8. Assumptions

- `friend_plus` token は v1 では multi-use bearer であり、one-time-use registry は持たない
- sponsor metadata は immediate sponsor のみ保持し、full sponsor chain は持たない
- `rotate` 後の new epoch は `open` で始まる
- rotation grant は author pubkey 単位で暗号化するため、same-author multi-device でも redeem できる
- rotate は current participant 集合を維持し、sharing frontier だけをリセットする
- `friend_of_friend` projection は suggestion / diagnostics 用であり、`friend_plus` の policy truth には使わない

## 9. Consequences

- `invite_only` は shipping baseline になる
- `friend_only` は hard-private として設計する
- `friend_plus` は soft-private として設計する
- `friend_plus` では participant mutual chain に沿った漏洩リスクを受け入れる
- `friend_plus` の rotate には encrypted fanout が必要になる
- ただしこの漏洩境界を超える変更は、実装時に別途確認が必要になる
- community-node は引き続き membership truth にならない

## 10. Decision Summary

2026-03-20 時点の `main` を前提にした更新後の判断は次のとおり。

- Phase 1 の `invite_only` はすでに実装済み
- `0013` の social graph foundation もすでに実装済み
- `friend_only` は owner-scoped `mutual` による hard-private として再設計する
- `friend_plus` は owner snapshot ではなく、participant-scoped `mutual` の連鎖による soft-private として再設計する
- `friend_plus` の v1 carrier は direct signed token、join 判定は import 時点、default TTL は 24h とする
- `friend_plus` の frontier stop は owner-only `freeze` と encrypted grant fanout を伴う `rotate` で扱う
- logical channel は stable に維持し、epoch は replica secret の切り替え単位として扱う

この方針なら、topic-first / docs-first / community-node control plane という現行境界を崩さずに、`friend_plus` を既存ユーザー期待とプロダクト意図に合わせて実装可能な設計へ落とし込める。
