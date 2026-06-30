# ADR 0026: Community Node Trust / Relation Foundation

## Status
Draft

## Date
2026-06-30

## Base Branch
`main`

## Related
- `docs/architecture/moderation-event-trust-semantics.md`（advisory ≠ command / authority scope / visibility）
- `docs/adr/0013-social-graph-foundation-draft.md`（author-owned follow graph。canonical, node-independent）
- `docs/adr/0025-community-node-indexing-foundation.md`（index = co-participation の観測元 / recommendation 境界）
- `docs/adr/0024-community-node-admission-data-classification.md`（admission。node-local な補助提供可否）
- `docs/safety/community-node-critical-safety.md`（§5 risk signal を trust/relation に反映 / §9 risk signal model）
- `crates/cn-safety/src/signal.rs`（`SafetyRiskSignal` / `RiskSignalTarget`）
- `crates/cn-operator/src/capability.rs`（`CommunityLocalTrust` = `Availability::Planned`）
- Issue: #406（runtime 結線で risk signal を trust/relation reads に反映）, #409（本 ADR）

## 位置づけ

community node の主要未検討機能 **trust / relation** の **意図・境界・read 契約**を固定する foundation ADR。`CommunityLocalTrust` capability（`Availability::Planned`）の中身を定義し、#406 の「risk signal を trust/relation reads に反映する」反映先 read 契約を確定する。

trust と relation は **node-local かつ advisory** な derived signal であり、`docs/architecture/moderation-event-trust-semantics.md` の不変条件（network-wide command ではない / issuer node の authority scope に閉じる / user identity・profile・social graph の canonical を所有・改変しない）に従う。

本 ADR は **意図・境界・read 契約**を定義する。具体的な clustering / scoring アルゴリズムの詳細は §6（未決）と後続 Issue に委ねる。

## Feature Data Classification

### trust
- Feature 名: community-local user trust signal（troll/bot でない確信度）
- Durable / Transient: Durable な node-local derived state（再構築可能）
- Canonical Source: derived。canonical な user trust は存在しない（node-independent）。入力は node-local 観測 + risk signal（#406, `cn_safety.risk_signals`）+ policy
- Replicated?: No（node-local advisory。配布する場合は trust-semantics の visibility に従う signal として）
- Rebuildable From: risk signals + node-local 観測 + policy。再計算可能
- Public Replica / Private Replica / Local Only: node-local（`cn-core` / Postgres）。配布は visibility（local / subscribed_nodes / public）規則
- Gossip Hint 必要有無: No
- Blob 必要有無: No
- SQLite projection 必要有無: No（server は Postgres）
- 必須 contract:
  - `trust_is_per_user_not_assertive_label`
  - `trust_reflects_risk_signals_as_weighted_input`
  - `trust_does_not_own_or_mutate_user_identity`
  - `trust_read_is_explainable_with_basis`
- 必須 scenario:
  - troll 的挙動 + risk signal がある pubkey の trust が下がり、basis 付きで説明できる / risk が無いと不当に下げない

### relation
- Feature 名: community-local pairwise relation（A↔B のコミュニティクラスタ近接度）
- Durable / Transient: Durable な node-local derived projection（再構築可能）
- Canonical Source: derived。canonical な relation は存在しない。入力は supported topic / 許可 channel の co-participation（index, ADR 0025 / #404）+ social-graph projection（ADR 0013, follow edges）
- Replicated?: No（node-local）
- Rebuildable From: index の co-participation + follow projection。再計算可能
- Public Replica / Private Replica / Local Only: node-local（`cn-core` / Postgres）
- Gossip Hint 必要有無: No
- Blob 必要有無: No
- SQLite projection 必要有無: No（server は Postgres）
- 必須 contract:
  - `relation_is_pairwise_cluster_proximity`
  - `relation_does_not_mutate_social_graph_canonical`
  - `relation_read_is_explainable`
  - `relation_visibility_choice_is_user_controlled_and_reversible`
  - `relation_does_not_auto_suppress_cross_cluster_content`
- 必須 scenario:
  - 同一 cluster の A,B は近接度が高く、別 cluster は低い（根拠つき）
  - user が opt-out すると他者の relation / discovery 表示に出ない（node-local。canonical 削除ではない）
  - cross-cluster content は user が選択しない限り自動で抑制されない

## 1. 背景と意図

### trust（信頼度）
- 定義: **その CN における、あるユーザーが troll / bot でない確信度**。SNS で一般的な bot 排除・荒らし排除の意図。
- node-local な derived signal。#406 の risk signal（`SafetyRiskSignal`、target = `UserPubkey` / `PeerNode`）を主要入力の一つとして反映する。

### relation（関係性 / クラスタ近接度）
- 定義: **CN から見た、ユーザー A とユーザー B のコミュニティクラスタ的な近さ**（A から見た B の近接度）。
- 目的:
  1. **public からコミュニティを構築していくきっかけづくり**（近接クラスタの surfacing）。
  2. **クラスタの内外を俯瞰的に定量化**し、対立 / エコーチェンバーに「ある種の納得感」を与える（descriptive transparency）。
  3. ユーザーに **「そもそも見ない / 見えない」という選択肢**を与え、コミュニティ間の大規模な対立を避ける（user agency）。

trust が「個人の信頼度（troll でないか）」を測るのに対し、relation は「2 者のクラスタ的距離」を測る。両者は別の量である。

## 2. Decision

### 2.1 trust と relation を別概念として定義する
- **trust**: per-user（pubkey）の node-local 信頼度。troll/bot でない確信度。断定ラベルではなく根拠つき（basis / 寄与 signal）advisory。対象は 1 者。
- **relation**: pairwise（viewer A, target B）の node-local クラスタ近接度。A↔B のコミュニティ的近さ。advisory。対象は 2 者間。
- 両者は別 read として提供し、混同しない。

### 2.2 social-graph v1 との境界 — overlay であって canonical を改変しない
- social-graph v1（ADR 0013）の follow edge / `mutual` / `friend_of_friend` は **author-owned canonical, node-independent**。
- trust / relation は CN が観測から導く **node-local な derived overlay** であり、social graph の canonical を所有・改変・上書きしない（trust-semantics の `does_not_apply_to: user_social_graph_canonical_source` と整合）。
- relation は follow projection を **入力の一つ**として参照してよいが、relation の出力は social graph とは別物（cluster proximity であって follow 関係ではない）。

### 2.3 trust の入力と read
- 入力（node-local, authority scope 内）: risk signal（#406, `cn_safety.risk_signals`、troll/bot/safety 系）、node-local な振る舞い観測（spam / abuse 報告、rate など）、policy。
- read: per-pubkey の trust signal（根拠つき。basis / 寄与 signal / confidence / visibility / expiry を説明可能）。
- 断定ラベル（「このユーザーは troll」）として出さず、根拠つき advisory として出す（trust-semantics §4 の説明責任）。

### 2.4 relation の入力と read
- 入力（node-local）: supported topic / 許可 channel の **co-participation**（index, ADR 0025 / #404 が観測元）、social-graph projection（follow edges, ADR 0013）、その他 node-local な共起シグナル。
- read: pairwise（A, B）の cluster proximity（近接度 + 根拠）。viewer 視点で相対化する。
- 具体的 clustering / scoring は §6（未決）。本 ADR は入力・出力形・境界のみ固定する。

### 2.5 advisory / authority scope / visibility（trust-semantics 準拠）
- trust / relation は network-wide command ではない。issuer CN の authority scope に閉じた optional trust input。
- 別 CN は別の trust / relation を持ちうる（単一の正解は無い）。
- 配布する場合は visibility（`local` / `subscribed_nodes` / `public`）に従う。誤検知を拡散しないため既定は `local` 寄り。
- client は issuer / basis / confidence / visibility / subscription を説明できる前提で表示する（断定ラベルを根拠なく出さない）。

### 2.6 user agency と echo-chamber への配慮
- relation はユーザーに **「見ない / 見えない」選択肢**を与えるための **descriptive かつ user-controlled** な signal とする。
- **CN は relation を使って cross-cluster content を勝手に抑制しない**（auto-segregation を既定にしない）。フィルタ / 非表示はユーザーの選択で発火し、可逆・説明可能であること。
- echo-chamber の緊張: 「見ない」選択は filter bubble を強める恐れがある。緩和として (a) 既定で隠さない、(b) 近接度と根拠を透明に提示し「納得感」を descriptive に与える、(c) 選択は可逆、(d) relation を「対立の固定化」ではなく「俯瞰と選択の材料」として位置づける。
- 「見えない」= ユーザーが自分を他者の relation / discovery 表示から opt-out できる（node-local。canonical 削除ではない）。

### 2.7 #406 の反映先 read 契約
- #406 の「risk signal を trust/relation reads に反映」は、本 ADR の **trust read への入力反映**として接続する（risk signal → trust の重み付け入力）。
- relation へは、risk が cluster の文脈に関わる場合に補助的に反映してよいが、relation の本体は cluster proximity であり risk ラベルではない。
- 反映は advisory（根拠つき）として行い、断定や canonical 改変はしない。

## 3. Consequences
- `CommunityLocalTrust` capability の中身が trust（per-user 信頼度）+ relation（pairwise cluster proximity）の 2 read として定義される。capability 昇格は実装・テストが揃った段階で別途判断（本 ADR では `Availability::Planned` 維持）。
- #406 は反映先 read（trust）が定義されたことでブロッカー解消。runtime 結線は trust read へ risk signal を入力する形になる。
- relation は index の co-participation を入力にするため、indexing（#404 / ADR 0025）に依存する。

## 4. Out of scope（後続 / 別 ADR・Issue）
- 具体的 clustering / community detection / scoring アルゴリズムと評価（§6）。
- trust scoring の具体式・閾値・decay。
- recommendation での relation 利用詳細（ADR 0025 の recommendation 境界に従う）。
- 決定論 / 非決定論 moderation の verdict 生成（#410 / #411）。
- client UI（trust / relation の表示・「見ない / 見えない」導線）。

## 5. 維持する境界
- trust / relation は node-local advisory。network-wide command でも canonical でもない。
- user identity / profile / social graph の canonical を所有・改変しない（overlay のみ）。
- 断定ラベルを根拠なく出さない（基準は trust-semantics §4）。
- relation で cross-cluster content を勝手に抑制しない（user 選択・可逆・透明）。
- 配布は visibility 規則に従い、誤検知を public へ拡散しない。

## 6. 未決事項（要設計・レビュー）
- relation の clustering / proximity の具体: 入力特徴（co-participation の重み、topic / channel 単位、時間減衰）、scoring、viewer 相対化、pairwise の計算 / 保存コスト。
- trust scoring の具体: risk signal の重み、behavior 特徴、decay / appeal（`AppealStatus`）反映、閾値。
- trust / relation の visibility 既定と配布の要否（per-user signal の配布はとくに慎重に）。
- 「見えない」opt-out の正確な意味（discovery 非表示の範囲、相互作用への影響）。
- private channel における trust / relation の扱い（admission / channel scope との関係、ADR 0024 / 0025 §6.3）。
