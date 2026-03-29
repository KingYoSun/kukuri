# ADR 0018: channel-first sidebar と unified epoch lifecycle

## Status
Accepted

## Date
2026-03-29

## Base Branch
`main`

## Related
- `docs/adr/0012-topic-first_progressive_community_filtering_draft.md`
- `docs/adr/0013-social-graph-foundation-draft.md`
- `docs/adr/0014-uiux-dev-flow.md`
- `docs/DESIGN.md`

## Context
- 現行 desktop shell では `channel` が `topic` の子である一方、UI 上は `Timeline / Channels / Live / Game / Profile` の並列 workspace として扱われていた。
- 実際には channel の切り替えは workspace ではなく scope の切り替えであり、`Timeline / Live / Game` の表示範囲と create target を変えるだけである。
- この不一致により、user は `topic -> channel -> workspace` の関係を短期記憶で補完する必要があり、`View Scope` / `Compose Target` / `Channels` tab / `Join Invite` / `Join Grant` / `Join Share` / `Create Invite` / `Create Grant` / `Create Share` / `Freeze` / `Rotate` などの冗長 control が増えていた。
- `0012` は 2026-03-20 時点の current main を固定する ADR として有効だが、そこにある `invite_only private channel data` の「hard-private baseline として継続利用」、`join grant / epoch control` の audience 別 split、desktop の `Channels` workspace と manual `Freeze` / `Rotate` は今回の UX 方向と衝突する。
- 参加者体験の観点では、新 epoch の share だけでなく invite も既存 participant には自動配布・自動適用される必要がある。owner が rotate や re-share のたびに participant へ manual join を要求する flow は許容しない。

## Decision

### 1. channel は topic 配下の選択状態として扱う
- `channel` は独立 workspace ではなく `topic` の child selection とする。
- primary route は `#/timeline`, `#/live`, `#/game`, `#/profile` に固定する。
- `#/channels` は廃止し、アクセス時は `#/timeline` へ normalize する。
- route/search param の正本は `topic` と optional `channel` とし、`timelineScope` と `composeTarget` は public route contract から外す。

### 2. channel switch は左サイドバーへ移す
- left rail を `status/settings -> ADD TOPIC -> CHANNEL -> Tracked Topics` に再編する。
- `Tracked Topics` は single-expand accordion とし、active topic 行の子要素としてその topic の joined channel 一覧を出す。
- topic 行の選択は public scope、channel 子要素の選択は private scope を表す。
- channel list の primary UI は `label + audience badge + selected state` に限定し、`epoch` / `sharing_state` / `joined_via` は debug か diagnostics に退避する。

### 3. Join / Share を unified action にする
- UI 上の import action は 1 つの `Join` に統一する。token kind の判定は runtime が内部で行い、invite / grant / share へ振り分ける。
- UI 上の export action は 1 つの `Share` に統一する。audience kind に応じた invite / grant / share の選択は runtime が内部で行う。
- `Join Invite`, `Join Grant`, `Join Share`, `Create Invite`, `Create Grant`, `Create Share` の個別 button は primary UI に置かない。
- `Freeze`, `Rotate`, `Close Sharing` の standalone button は primary UI に置かない。

### 4. 全 audience を epoch-aware lifecycle に揃える
- `invite_only`, `friend_only`, `friend_plus` のすべてを epoch-aware private channel として扱う。
- signed object の正本は `channel-policy`, `channel-participant`, `channel access token`, `epoch handoff grant` に揃える。
- `PrivateChannelRotationGrant*` は audience 非依存の epoch handoff grant 契約へ一般化する。
- owner の write/share 前には必要に応じて auto cutover を実行する。
  - `invite_only`: write/share 前に current epoch を seal し、new epoch へ rotate する。
  - `friend_only`: stale participant を評価し、必要時のみ rotate する。eligible mutual participant にだけ handoff する。
  - `friend_plus`: write/share 前に current epoch を seal し、new epoch へ rotate する。

### 5. existing participant への auto distribution / auto apply を正本にする
- owner が rotate または share したとき、current participant には新 epoch handoff grant を自動配布する。
- この自動 handoff は audience 別に次の意味を持つ。
  - `invite_only`: current participant 全員に新 epoch access を自動配布する。manual re-invite は不要。
  - `friend_only`: current participant のうち current policy を満たす相手にだけ新 epoch access を自動配布する。
  - `friend_plus`: current participant 全員に新 epoch access を自動配布する。
- participant 側は background refresh, scoped read, joined channel list refresh, restart restore のいずれからでも同じ redemption path で新 epoch を自動適用する。
- 既存 participant に対する invite 再配布も同じ auto distribution / auto apply contract に含める。user-facing には `Join` 再入力を要求しない。

### 6. legacy invite_only は互換移行しない
- 既存の legacy `invite_only` channel は epoch-aware へ in-place migration しない。
- upgrade 後に legacy `invite_only` participant が継続参加するには fresh invite が必要とする。
- 新規作成される `invite_only` channel は常に epoch-aware 初期 epoch を持つ。

## Implementation Contract

### Desktop shell
- left rail に `CHANNEL` section を持たせ、channel 作成、token 入力、`Join`、selected channel 向け `Share` を置く。
- `Timeline` / `Live` / `Game` は同じ topic/channel selection を正本にして list/create を scope する。
- `Profile` は active topic の public self timeline を表示し、current channel は復帰用 state にのみ使う。

### Route / state
- `PrimarySection` は `timeline | live | game | profile` に限定する。
- invalid path と legacy `#/channels` は `#/timeline` に replace normalize する。
- private selection の保存は `selectedChannelIdByTopic` を正本にし、API 呼び出し時だけ `public` または `channel:<id>` へ変換する。

### Runtime / frontend API
- frontend/runtime には `importChannelAccessToken(token)` と `exportChannelAccessToken(topicId, channelId, expiresAt)` を追加する。
- import result は `kind` を持つ discriminated union とし、`topic_id`, `channel_id`, `channel_label`, `epoch_id` を共通 field とする。
- export result は `kind` と `token` を返すが、UI copy は `kind` を primary label に露出しない。

### Domain
- `invite_only` を含む private audience はすべて `channel-policy` と `channel-participant` に参加する。
- auto rotate は handoff grant の配布までを 1 operation として扱う。
- participant redeem は read path から呼べる idempotent operation とする。

## Consequences
- channel を workspace と見なす実装は以後の正本ではなくなる。`Channels` tab や route を前提にした UI は削除対象になる。
- user は `topic -> channel -> workspace` を視覚的に追えるようになり、`Timeline / Live / Game` の scope change が left rail selection と一致する。
- owner control は減るが、runtime の epoch orchestration は増える。rotate/re-share の correctness は UI ではなく domain test と harness で守る必要がある。
- `invite_only` も epoch-aware になったため、old invite token と old epoch capability を前提にした test は fresh invite 前提へ更新が必要になる。
- この ADR と衝突する場合、`0012` にある次の前提より本 ADR を優先する。
  - invite-only を legacy hard-private baseline とする前提
  - audience 別に invite/grant/share/export UI を分ける前提
  - manual `Freeze` / `Rotate` を user-facing control とする前提
