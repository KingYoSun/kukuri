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

## 1. Context

2026-03-19 時点の `main` を確認した結果、現行 kukuri はすでに strong な topic-first 構成を持っている。

- desktop UI は `trackedTopics + activeTopic + timelinesByTopic` を中心に動く
- `crates/app-api` は topic ごとに 1 本の購読タスクを持ち、docs replica と hint topic を同時に監視する
- 実データは `iroh-docs` / `iroh-blobs`、通知は `iroh-gossip`、接続補助は static-peer / seeded DHT / community-node を使う
- community-node は auth / consent / bootstrap / relay assist の control plane であり、topic ごとの ACL サービスではない

この前提の上で、「topic-first を維持したままコミュニティ範囲を段階的に絞り込めるか」を再評価する。

## 2. Current Main Snapshot

### 2.1 topic-first の実装状態

- `crates/app-api/src/lib.rs` の `ensure_topic_subscription` / `spawn_topic_subscription` は topic 単位で 1 replica と 1 hint stream を購読する
- 投稿作成は `create_post_with_attachments` から始まり、topic をキーに docs と hints へ流れる
- desktop UI でも topic は `Add Topic -> Tracked Topics -> active topic -> Timeline` の一直線な導線になっている

結論として、topic-first 自体はすでに成立している。新機能はこの軸を壊さず、topic の下に絞り込み単位を増やすべきである。

### 2.2 現時点で access control になっていないもの

- `crates/core/src/lib.rs` には `ObjectVisibility` がある
- ただし現行の投稿作成は `ObjectVisibility::Public` 固定で、visibility は metadata に留まる
- `crates/docs-sync/src/lib.rs` の `replica_secret` は `ReplicaId` から決定的に導出される

したがって現行 main では、

- visibility を `community` や `private` に変えても同期先は変わらない
- replica_id が分かれば同じ namespace secret を再現できる
- 実効的な access control はまだ存在しない

### 2.3 community-node の最新状態

community-node まわりの最新 main 実装は、以前の「restart 前提の relay 補助」より進んでいる。

- `crates/cn-user-api` は auth challenge / verify、consent、bootstrap node 配布、endpoint heartbeat を提供する
- `crates/cn-core` の bootstrap peer registration は subscriber 単位ではなく endpoint 単位で TTL 管理される
- `crates/desktop-runtime` は consent 後に runtime connectivity assist を current session に再適用できる
- `apps/desktop/src/App.test.tsx` でも `Save Nodes -> Authenticate -> Accept` 後に `active on current session` になることを確認している

ただし community-node は今も topic 別 membership を持たない。返しているのは「認証済み subscriber に対する bootstrap node / relay URL / seed peers」であり、friend-only や invite-only の正本にはなっていない。

## 3. Gap Analysis

| 項目 | current main | 段階的コミュニティ絞り込みに対する意味 |
|---|---|---|
| topic データ面 | topic ごとに 1 docs replica + 1 hint topic | 絞り込みには topic 配下の channel 抽象が必要 |
| replica secret | `ReplicaId` から決定的導出 | private channel をそのままは作れない |
| visibility | metadata のみ | access control の enforcement には使えない |
| community-node | global な auth / consent / bootstrap | topic 別 ACL や invite registry には使えない |
| social graph | 未実装 | friend-only / friend-plus を名前どおりには定義できない |
| desktop UX | topic 切替と composer は単純 | 追加導線は topic-first を壊さない配置が必要 |

## 4. Feasibility Review

### 4.1 invite-only は実現可能

invite-only は、現行 main の延長で最も現実的に実装できる。

必要なのは social graph ではなく capability ベースの private channel である。

- topic の下に opaque な private channel を追加する
- private channel の replica secret は決定的導出ではなく、ランダム生成または import した capability から得る
- hints も topic 名から推測できる公開文字列ではなく、channel 専用の opaque topic を使う
- 参加は invite import で行い、desktop が secret を secure storage に保存する

これは `docs-sync` / `app-api` / `desktop-runtime` の拡張で閉じる。community-node は relay assist と seed 配布だけでよい。

### 4.2 friend-only は今の main では未成立

現行コードベースには、friend-only を定義するための前提がない。

- contacts / follow graph がない
- mutual 判定の canonical source がない
- membership revocation / epoch rotation の仕組みがない
- UI 上も「誰に見えるのか」を説明する根拠がない

したがって current main に対して friend-only を入れる場合、実際には「相互フォロー限定」ではなく「明示メンバー限定」になる。これは friend-only という名前より member-list / approved-members の設計である。

### 4.3 friend-plus は今の main では非現実的

friend-plus を friend-of-friends として扱うなら、少なくとも以下が必要になる。

- social graph の durable state
- depth-2 展開の計算とキャッシュ
- Sybil / spam 対策
- join request / approval / revocation の UX

現行 main はそこまでの前提を持っていない。protocol draft (`docs/adr/0011-kukuri-protocol-v1-draft.md`) に community / membership object はあるが、実装本体にはまだ入っていない。

結論として、friend-plus はこの ADR の immediate scope に置くべきではない。

## 5. Updated Design Direction

### 5.1 即時に狙うべき範囲

この ADR では、段階的コミュニティ絞り込みの第一段階を次のように定義し直す。

1. `public`
2. `invite_only`
3. `member_list` または `approval_only` を将来拡張として追加
4. `friend_only` / `friend_plus` は social graph 導入後に再定義する

つまり、最初に実装すべきなのは「friend 系 audience そのもの」ではなく、topic 配下に private channel を増設できる基盤である。

### 5.2 channel を第一級概念にする

topic-first を壊さないために、追加するのは topic の置き換えではなく topic の下位 channel である。

例:

```text
topic: kukuri:topic:contract
  - public channel
  - invite-only channel A
  - invite-only channel B
```

このとき、

- topic は発見・一覧・ユーザー認知の軸
- channel は同期範囲・通知範囲・secret 管理の軸

と分離する。

### 5.3 enforcement は visibility ではなく capability で行う

実際の access control は以下で担保する。

- private replica は決定的 secret を使わない
- private hint topic は opaque identifier を使う
- desktop は imported capability を secure storage へ保存する
- channel 未参加の client は replica も hint も知らない

`ObjectVisibility` は「この投稿は public か invite-only か」を UI や object metadata に反映する補助情報としては使えるが、access control の正本にはしない。

### 5.4 community-node の責務は広げない

current main の方針どおり、community-node は control plane に留める。

- relay assist
- bootstrap node 配布
- consent / policy
- authenticated seed peer registration

invite-only の membership や doc capability の正本を community-node に置かない。そうしないと `docs + blobs + hints` が正本という現行構造を崩す。

## 6. Required Implementation Work

### 6.1 `crates/core`

必要最小限の追加は以下。

- `ChannelId` または `AudienceChannelId`
- `PostAudience` または `ChannelRef`
- post header に「どの channel に属するか」を表す field

ここで friend-only / friend-plus を固定 enum にすると後で意味論がぶれる。最初は `public` と `private channel ref` を表現できれば十分。

### 6.2 `crates/docs-sync`

最重要変更点。

- public replica は現在どおり deterministic secret を使ってよい
- private replica は local secret store または imported capability から開く
- `DocsSync` に private replica import/export の API が必要
- current `ReplicaId -> NamespaceSecret` 一発導出モデルを public 専用へ限定する

これがない限り invite-only は成立しない。

### 6.3 `crates/app-api`

現在の `topic -> subscription task 1本` を、`topic + channel` へ拡張する必要がある。

- `spawn_topic_subscription` は public channel だけでなく joined private channels も扱う
- hydration は channel ごとに docs を引き、topic projection に反映する
- reply は親投稿の audience を継承する
- sync diagnostics も topic 単位だけでなく channel 由来を内包できるようにする

特に reply の audience 継承を最初から入れないと、private thread へ public reply を誤送信する UX 事故が起きやすい。

### 6.4 `crates/desktop-runtime`

- private channel capability の secure storage
- invite import / export API
- restart 後の private channel 復元

current main は discovery seed や community-node relay assist を current session に再適用できるので、invite-only でも connectivity 補助は runtime rebuild で扱える。

### 6.5 `apps/desktop`

UI は topic-first のまま、audience の選択と参加導線だけを追加する。

- topic header に audience switcher
- composer に現在の audience 表示
- invite import 導線
- joined private channels の状態表示

community-node panel の中に invite UX を混ぜない。あの panel は infra 設定であり、一般ユーザー向けの参加導線としては重すぎる。

## 7. UX Review

### 7.1 使いやすい導線にする条件

current main の UX で強いのは「topic を選んで、すぐ見る / 書く」が一貫している点である。これを壊さないことが最優先になる。

避けるべきもの:

- composer の中央に常時 3 種類の scope radio を置く
- community-node 設定と invite 参加を同じ panel に混ぜる
- public / private の timeline を無差別に 1 本へ混ぜ、送信先が分かりにくくなる状態

### 7.2 推奨導線

推奨するのは次の流れ。

1. ユーザーは従来どおり topic を追加・選択する
2. topic header に `Audience` を置く
3. 表示モードは `Public` / `Joined private channels` / `All joined` のいずれかを選べる
4. composer は現在選択中の audience を明示する
5. reply は親投稿の audience を継承し、明示的 override がない限り広げない

これなら topic-first を保ったまま、誤投稿リスクも下げられる。

### 7.3 invite join 導線

invite-only の参加導線は別入口にする。

推奨:

1. `Join via Invite` を topic 追加導線の近くに置く
2. invite import 後に `topic label / inviter / expires_at` を確認する
3. 承認後に該当 topic を `Tracked Topics` へ自動追加する
4. private channel がある topic であることを topic list に表示する

これにより「招待を受けたのにどこへ行けばよいか分からない」状態を避けられる。

### 7.4 terminology

内部設計では `friend_plus` / `friend_only` を議論してよいが、UI ラベルとしては直ちに採用しない方がよい。

理由:

- 現在の実装には friend graph がない
- ユーザーにとって「friend-only」と表示されても判定根拠が説明できない
- 実際の first ship は invite / member list ベースになる可能性が高い

UI ではまず `Public` / `Invite only` / `Approved members` のような説明可能なラベルを使うべきである。

## 8. Recommended Rollout

### Phase 1

invite-only private channel foundation

- private replica capability
- opaque hint topic
- invite import / export
- topic-first UI 上の audience switcher

### Phase 2

member-list / approval-based channel

- explicit membership docs
- revocation
- epoch rotation

### Phase 3

friend semantics の再評価

- contacts / follow graph
- mutual 判定
- friend-of-friends
- spam / abuse controls

## 9. Test Strategy

この機能は必ず failing test 先行で入れる。

最低限必要なもの:

- docs-sync: private replica を import した peer だけが同期できる
- app-api: invite-only channel では public hint を購読していない peer に投稿が見えない
- app-api: private thread への reply が audience を継承する
- desktop-runtime: restart 後に imported private channel capability を復元できる
- desktop frontend: invite import 後に topic が追加され、audience が明示される
- regression: current public topic sync、seeded DHT、community-node public connectivity が壊れない

## 10. Decision

2026-03-19 時点の `main` を前提にすると、段階的コミュニティ絞り込みは次のように判断する。

- invite-only は feasible
- member-list / approval-only は private channel 基盤の上で feasible
- friend-only は social graph 導入前は未成立
- friend-plus は immediate scope から外す

したがって本 ADR は、従来の「friend-plus -> friend-only -> invite-only をまとめて実装する計画」から、以下へ更新する。

- まず topic 配下の private channel 基盤を実装する
- first ship は invite-only を狙う
- friend 系 audience は protocol / membership foundation が入ってから別 ADR で再定義する

これが current main に対して最も整合的で、かつユーザー導線も壊しにくい。
