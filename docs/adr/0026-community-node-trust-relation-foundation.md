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
- Canonical Source: derived。canonical な user trust は存在しない（node-independent）。**絶対成分**の入力は決定論（#410）+ 厳格非決定論（#411）の verdict / risk signal（`cn_safety.risk_signals`）、**相対成分**の入力は非決定論（#411）+ node-local 観測 + relation。policy で合成
- Replicated?: No（node-local advisory。配布する場合は trust-semantics の visibility に従う signal として）
- Rebuildable From: moderation verdict / risk signals + node-local 観測 + relation + policy。再計算可能
- Public Replica / Private Replica / Local Only: node-local（`cn-core` / Postgres）。配布は visibility（local / subscribed_nodes / public）規則
- Gossip Hint 必要有無: No
- Blob 必要有無: No
- SQLite projection 必要有無: No（server は Postgres）
- 必須 contract:
  - `trust_is_not_single_absolute_scalar`
  - `trust_separates_absolute_and_relative_indicators`
  - `trust_absolute_indicators_not_relation_weighted`
  - `trust_relative_indicators_are_relation_weighted`
  - `trust_resists_mass_report_bombing`
  - `trust_absolute_negative_is_weighted_double`
  - `trust_composition_weights_are_operator_tunable`
  - `trust_reflects_risk_signals_split_by_category`
  - `trust_does_not_own_or_mutate_user_identity`
  - `trust_read_is_explainable_with_basis`
- 必須 scenario:
  - CSAM 系 risk がある pubkey は絶対成分が下がり、relation や通報数で揺れない
  - 特定 cluster からの大量通報は相対成分に raw count として効かず、relation で重み付けされる（report-bombing 耐性）
  - hate / アダルトなど相対指標は viewer の cluster によって評価が変わる / risk・観測が無ければ不当に下げない

### relation
- Feature 名: community-local pairwise relation（A↔B のコミュニティクラスタ近接度）
- Durable / Transient: Durable な node-local derived projection（再構築可能）
- Canonical Source: derived。canonical な relation は存在しない。入力は supported topic / 許可 channel の co-participation（index, ADR 0025 / #404）+ social-graph projection（ADR 0013, follow edges）。relation graph backend は ArcadeDB（最小）/ neo4j（scale, Cypher 互換, §6.1）
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
- **trust は単一の絶対スカラーにしない。** コミュニティ単位の通報爆撃（恣意的な大量通報）が予想される以上、CN にとっての**絶対指標**（CSAM など community / culture に依らない）と、**相対指標**（hate / 暴力 / アダルトなど community / 文化圏で法・判断が変わる）を分けて処理する。絶対指標は決定論 + 厳格非決定論で、相対指標は relation（clustering）を加味した値で扱う（§2.3）。

### relation（関係性 / クラスタ近接度）
- 定義: **CN から見た、ユーザー A とユーザー B のコミュニティクラスタ的な近さ**（A から見た B の近接度）。
- 目的:
  1. **public からコミュニティを構築していくきっかけづくり**（近接クラスタの surfacing）。
  2. **クラスタの内外を俯瞰的に定量化**し、対立 / エコーチェンバーに「ある種の納得感」を与える（descriptive transparency）。
  3. ユーザーに **「そもそも見ない / 見えない」という選択肢**を与え、コミュニティ間の大規模な対立を避ける（user agency）。

trust が「個人の信頼度（troll でないか）」を測るのに対し、relation は「2 者のクラスタ的距離」を測る。両者は別の量である。

## 2. Decision

### 2.1 trust と relation を別概念として定義する
- **trust**: per-user（pubkey）の node-local 信頼度。troll/bot でない確信度。**単一の絶対スカラーにせず、絶対指標 + 相対指標の合成**として扱う（§2.3）。断定ラベルではなく根拠つき（basis / 寄与 signal）advisory。
- **relation**: pairwise（viewer A, target B）の node-local クラスタ近接度。A↔B のコミュニティ的近さ。advisory。対象は 2 者間。
- 両者は別 read として提供し、混同しない。

### 2.2 social-graph v1 との境界 — overlay であって canonical を改変しない
- social-graph v1（ADR 0013）の follow edge / `mutual` / `friend_of_friend` は **author-owned canonical, node-independent**。
- trust / relation は CN が観測から導く **node-local な derived overlay** であり、social graph の canonical を所有・改変・上書きしない（trust-semantics の `does_not_apply_to: user_social_graph_canonical_source` と整合）。
- relation は follow projection を **入力の一つ**として参照してよいが、relation の出力は social graph とは別物（cluster proximity であって follow 関係ではない）。

### 2.3 trust の構成 — 絶対指標と相対指標を分ける（report-bombing 耐性）
- trust は **単一の絶対スカラーにしない**。コミュニティ単位の通報爆撃（恣意的な大量通報）が予想される以上、CN にとっての絶対指標と、コミュニティ / 文化圏で法・判断が変わる相対指標を分けて処理する。
- **絶対指標（absolute）**: CSAM など、community / culture に依らず CN にとって絶対の指標。
  - 入力: **決定論的 moderation（#410）+ 厳格な非決定論的 moderation（#411）**（known-hash / provider-verdict / 厳格 classifier）。
  - **relation で重み付けしない**。evidence / 検知ベースであり、通報数では動かない（report-bombing に対して不動）。
- **相対指標（relative）**: hate / 暴力 / アダルトなど、community / 文化圏で法・判断が変わる指標。
  - 入力: 非決定論的 moderation（#411）+ node-local 観測（spam / abuse 報告、rate など）+ **relation（clustering）を加味**した値。
  - viewer / cluster 相対。relation で重み付けすることで、特定 cluster からの大量通報が raw count として効かず cluster 文脈で相対化される（report-bombing 耐性）。
- bot / 自動化 abuse の検知は挙動ベースの絶対指標寄り、troll / harassment の判断は文化依存の相対指標寄りとして扱う。
- read: trust は **絶対成分（viewer 非依存）+ 相対成分（viewer / cluster 依存）**の合成として返す（合成式は §6.2、初期決め打ち・operator 可変）。いずれも断定ラベル（「このユーザーは troll」）ではなく根拠つき advisory（basis / 寄与 signal / confidence / visibility / expiry を説明可能、trust-semantics §4）。

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

### 2.7 #406 の反映先 read 契約（絶対 / 相対で振り分ける）
- #406 の「risk signal を trust/relation reads に反映」は、risk signal の category に応じて trust の **絶対成分 / 相対成分**へ振り分けて反映する。
  - CSAM など critical safety = **絶対成分**（relation 非依存、report-bomb で不動）。
  - hate / nsfw / spam など = **相対成分**（relation で重み付け、viewer / cluster 相対）。
- relation は相対成分の重み付け入力として使う。relation 本体は cluster proximity であり risk ラベルではない。
- 反映は advisory（根拠つき）。断定 / canonical 改変はしない。

## 3. Consequences
- `CommunityLocalTrust` capability の中身が trust（per-user 信頼度）+ relation（pairwise cluster proximity）の 2 read として定義される。capability 昇格は実装・テストが揃った段階で別途判断（本 ADR では `Availability::Planned` 維持）。
- #406 は反映先 read（trust）が定義されたことでブロッカー解消。runtime 結線は risk signal を category に応じて trust の絶対 / 相対成分へ入力する形になる。
- trust が絶対 / 相対の 2 系統に分かれるため、絶対系は決定論（#410）+ 厳格非決定論（#411）moderation に、相対系は #411 + relation に依存する。本 ADR は #410 / #411 と密接に結びつく。
- relation は index の co-participation を入力にするため、indexing（#404 / ADR 0025）に依存する。graph backend は最小 = ArcadeDB、scale = neo4j（Cypher 互換、§6.1）。

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

## 6. 設計方針（最小構成）と未決事項

### 6.1 relation clustering の backend（規模とコストで選択）
- 判断軸は **規模とコスト**。
- **最小構成**: **ArcadeDB**（軽量・低コスト・埋め込み可能な multi-model / graph）。項目ごとの解析 worker が複数項目を点数化（例: 共有 topic 数、friend-of-friend、co-participation 頻度、follow projection）して relation graph に格納し、pairwise proximity を graph query で算出する。relation の観測元（co-participation）は index（Postgres, ADR 0025）。
- **scale path**: **neo4j**（大規模）。ArcadeDB と **Cypher 互換**で移行しやすい。
- **graph-store 抽象境界**を置き、ArcadeDB ↔ neo4j を差し替え可能にする。

### 6.2 trust scoring（絶対 / 相対の合成）— 初期は決め打ち、operator 可変
- 絶対成分・相対成分はそれぞれ **±1 を上下限**とする。
  - 絶対成分: 決定論（#410）+ 厳格非決定論（#411）の verdict / risk signal から算出。relation 非依存、report-bomb 不動。
  - 相対成分: 非決定論（#411）+ node-local 観測を relation で重み付けして算出。viewer / cluster 相対。
- **合成式（初期決め打ち）**:
  ```
  absolute ∈ [-1, 1], relative ∈ [-1, 1]
  w_abs = 2.0 if absolute < 0 else 1.0
  trust = (w_abs * absolute + relative) / 2
  ```
  絶対成分がマイナス（CSAM など確定的に排除すべき）のとき重み 2 倍で distrust を支配的にし、相対指標（文化依存）が良好でも薄まらないようにする。プラスのときは 1 倍。
- **重みは operator が変更できる**（`w_abs` の係数等）。
- 注: 絶対成分がマイナスのとき `trust` は -1 を下回り得る（distrust を強く効かせる意図）。最終クランプの要否は未決。
- 閾値・decay / appeal（`AppealStatus`）反映の具体は未決。

### 6.3 その他の未決事項
- relation の入力特徴の重み（co-participation / topic・channel 単位 / 時間減衰）、scoring、viewer 相対化、pairwise の計算 / 保存コスト。
- trust / relation の visibility 既定と配布の要否（per-user signal の配布はとくに慎重に）。
- 「見えない」opt-out の正確な意味（discovery 非表示の範囲、相互作用への影響）。
- private channel における trust / relation の扱い（admission / channel scope との関係、ADR 0024 / 0025 §6.3）。
