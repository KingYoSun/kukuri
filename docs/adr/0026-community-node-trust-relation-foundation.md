# ADR 0026: Community Node Trust / Relation Foundation

## Status
Accepted（foundation は #409 / PR #414 で確定。§6 未決事項は #416 で Decision 化）

## Date
2026-06-30

## Base Branch
`main`

## Related
- `docs/adr/0027-deterministic-moderation-critical-safety.md`（advisory ≠ command / authority scope / visibility, §2.1 / §2.6。旧 `moderation-event-trust-semantics.md` を集約）
- `docs/adr/0013-social-graph-foundation-draft.md`（author-owned follow graph。canonical, node-independent）
- `docs/adr/0025-community-node-indexing-foundation.md`（index = co-participation の観測元 / recommendation 境界）
- `docs/adr/0024-community-node-admission-data-classification.md`（admission。node-local な補助提供可否）
- `docs/adr/0027-deterministic-moderation-critical-safety.md`（§2.5 / §2.6 signed event / risk signal model）
- `crates/cn-safety/src/signal.rs`（`SafetyRiskSignal` / `RiskSignalTarget`）
- `crates/cn-operator/src/capability.rs`（`CommunityLocalTrust` = `Availability::Planned`）
- Issue: #406（runtime 結線で risk signal を trust/relation reads に反映）, #409（本 ADR foundation）, #416（§6 未決事項の Decision 化）

## 位置づけ

community node の主要未検討機能 **trust / relation** の **意図・境界・read 契約**を固定する foundation ADR。`CommunityLocalTrust` capability（`Availability::Planned`）の中身を定義し、#406 の「risk signal を trust/relation reads に反映する」反映先 read 契約を確定する。

trust と relation は **node-local かつ advisory** な derived signal であり、`docs/adr/0027-deterministic-moderation-critical-safety.md` §2.1 の不変条件（network-wide command ではない / issuer node の authority scope に閉じる / user identity・profile・social graph の canonical を所有・改変しない）に従う。

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
  - `trust_is_clamped_to_unit_interval`（最終値は `[-1, 1]`, §6.2）
  - `trust_absolute_component_does_not_decay`（絶対成分は時間減衰しない, §6.2）
  - `trust_relative_component_decays_over_time`（相対成分・node-local 観測は半減期減衰, §6.2）
  - `trust_appeal_pending_holds_contribution`（`pending` は寄与据え置き, §6.2）
  - `trust_appeal_accepted_excludes_contribution`（`accepted` は該当寄与除外, §6.2）
  - `cross_node_pull_discloses_only_confirmed_absolute_component`（cross-node pull は confirmed 絶対成分のみ, §6.3）
  - `viewer_relative_read_requires_authenticated_viewer`（相対成分 read は viewer 署名検証必須, §6.3）
- 必須 scenario:
  - CSAM 系 risk がある pubkey は絶対成分が下がり、relation や通報数で揺れない
  - 特定 cluster からの大量通報は相対成分に raw count として効かず、relation で重み付けされる（report-bombing 耐性）
  - hate / アダルトなど相対指標は viewer の cluster によって評価が変わる / risk・観測が無ければ不当に下げない
  - 絶対成分と相対成分がともに最悪でも最終 `trust` は `-1` を下回らない（clamp）
  - 相対成分の寄与は時間経過で半減し、絶対成分（known-hash 由来）は同期間で減衰しない
  - 他 node が pull すると confirmed 絶対成分のみ根拠つきで返り、相対成分は返らない
  - viewer 署名の無い / 別 identity を騙る viewer 相対 read は拒否される（なりすまし防止）

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
  - `relation_read_requires_authenticated_viewer`（viewer 相対 read は viewer 署名検証必須, §6.3）
  - `relation_opt_out_hides_from_others_relation_and_discovery`（opt-out で他者の relation read / discovery 双方に出ない, §6.3）
  - `relation_defaults_local_and_not_cross_node_pullable`（relation は `Local` 既定, cross-node pull に返さない, §6.3）
  - `relation_private_channel_signal_stays_local_and_scoped`（private channel 由来は channel + CN authority に閉じ Local 固定, §6.3）
- 必須 scenario:
  - 同一 cluster の A,B は近接度が高く、別 cluster は低い（根拠つき）
  - user が opt-out すると他者の relation / discovery 表示に出ない（node-local。canonical 削除ではない）。opt-out は trust には影響しない
  - cross-cluster content は user が選択しない限り自動で抑制されない
  - relation read は cross-node pull に返らない（`Local` 既定）
  - private channel 由来の co-participation は channel メンバー可視の relation に閉じ、public relation に混ざらない

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
- **read は pull 型**。ある pubkey について read API を叩くと、**その CN の視点**で計算された trust / relation が返る（push 複製ではない。§6.3 で Decision 化）。node A が node B に問い合わせれば返るのは **B の view** であり、A 自身の view は A の CN が返す。
- `visibility`（`local` / `subscribed_nodes` / `public`）は **push 複製ポリシーではなく、cross-node pull に対するアクセス範囲**（誰がこの CN の signal を read できるか）として解釈する。誤検知を拡散しないため既定は `local`（§6.3）。
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
- #416 で §6 未決事項を Decision 化: 最終クランプ `[-1,1]`（§6.2）、閾値なし / 相対成分 decay / appeal 反映（§6.2）、pull 型 read と cross-node 開示は confirmed 絶対成分のみ + viewer 相対 read の署名検証（§6.3）、opt-out 範囲（§6.3）、private channel は Local 固定（§6.3）、graph-store 抽象境界 API と移行閾値（§6.1）。foundation 実装（#415）はこれらの contract / scenario を満たす。

## 4. Out of scope（後続 / 別 ADR・Issue）
- 具体的 clustering / community detection アルゴリズムの評価・チューニング（入力特徴の重み実測、viewer 相対化の詳細、pairwise の計算 / 保存コスト最適化）。合成式・最終クランプ・decay・appeal・visibility の方針は §6 で決定済み。
- recommendation での relation 利用詳細（ADR 0025 の recommendation 境界に従う）。
- 決定論 / 非決定論 moderation の verdict 生成（#410 / #411）。
- client UI（trust / relation の表示・「見ない / 見えない」導線）。
- foundation 実装（#415）。本 ADR は決定・contract を固定し、実装は #415 が担う。

## 5. 維持する境界
- trust / relation は node-local advisory。network-wide command でも canonical でもない。
- user identity / profile / social graph の canonical を所有・改変しない（overlay のみ）。
- 断定ラベルを根拠なく出さない（基準は trust-semantics §4）。
- relation で cross-cluster content を勝手に抑制しない（user 選択・可逆・透明）。
- read は pull 型。cross-node pull への開示は visibility 規則に従い、confirmed 絶対成分のみに限る（相対成分・relation は Local）。誤検知を public へ拡散しない。
- viewer 相対 read は viewer identity の署名検証を必須とし、なりすましによる他者視点の取得を許さない。

## 6. Decision（§6 未決事項の Decision 化, #416）

foundation（#409 / PR #414）が残した §6 の未決事項を #416 で決定した。以下を Decision とする。なお具体的 clustering / community detection アルゴリズムの評価・チューニング（入力特徴の重み実測、pairwise 計算 / 保存コストの最適化）は後続 Issue に残る（§4 Out of scope）。

### 6.1 relation clustering の backend と graph-store 抽象境界（Decision）
- **最小構成 = ArcadeDB**（軽量・低コスト・埋め込み可能な multi-model / graph）。項目ごとの解析 worker が複数項目を点数化（例: 共有 topic 数、friend-of-friend、co-participation 頻度、follow projection）して relation graph に格納し、pairwise proximity を graph query で算出する。relation の観測元（co-participation）は index（Postgres, ADR 0025）。
- **scale path = neo4j**（大規模）。ArcadeDB と **Cypher 互換**で移行しやすい。
- **graph-store 抽象境界（Decision）**: backend を差し替え可能にする trait を置く。最小 API 形は次を必須とする（いずれも viewer / target は pubkey、proximity は根拠つき）:
  - `upsert_edge(from, to, features)` — co-participation / follow projection 等の特徴を格納。
  - `pairwise_proximity(viewer, target) -> Proximity{ score, basis }` — A 視点の B への近接度（viewer 相対、根拠つき）。
  - `neighbors(viewer, k) -> [pubkey]` — discovery / surfacing 用の近接近傍。
  - `cluster_of(pubkey) -> ClusterRef` — cluster 帰属（相対成分の重み付け入力）。
  - クエリは **Cypher 互換**表現に落とせることを前提とし、ArcadeDB / neo4j 双方で同一 API を満たす。
- **移行閾値（Decision）**: 判断軸は **規模 + profile**。
  - invite-only（ADR 0024 admission）/ 小規模 node は **ArcadeDB 据え置き**。
  - public profile かつ大規模 supported topic を持つ node は edge / node 数の増大で **neo4j を検討**。
  - 具体しきい値は運用データが無いため **初期は目安に留め、`cn-operator` の readiness で再評価**する（決め打ち固定値は置かない）。

### 6.2 trust scoring（絶対 / 相対の合成）— 最終クランプ・閾値・decay・appeal（Decision）
- 絶対成分・相対成分はそれぞれ **±1 を上下限**とする。
  - 絶対成分: 決定論（#410）+ 厳格非決定論（#411）の verdict / risk signal から算出。relation 非依存、report-bomb 不動、**viewer 非依存**（§6.3 の pull 開示で viewer context を要さない成分）。
  - 相対成分: 非決定論（#411）+ node-local 観測を relation で重み付けして算出。**viewer / cluster 相対**。
- **合成式（初期決め打ち, operator 可変）**:
  ```
  absolute ∈ [-1, 1], relative ∈ [-1, 1]
  w_abs = 2.0 if absolute < 0 else 1.0
  trust = clamp(-1, 1, (w_abs * absolute + relative) / 2)
  ```
  絶対成分がマイナス（CSAM など確定的に排除すべき）のとき重み 2 倍で distrust を支配的にし、相対指標（文化依存）が良好でも薄まらないようにする。プラスのときは 1 倍。
- **最終クランプ（Decision）**: `trust` は最終的に **`[-1, 1]` にクランプ**する。絶対成分マイナス時に合成値が -1 を下回り得る（例: absolute=-1, relative=-1 → -1.5）が、read 値域を対称に保ち client 表示・閾値設計を単純化するため clamp する。distrust の支配性は「絶対マイナス時は relative が良好でも救済されない」性質で clamp 前に既に担保される。
- **重みは operator が変更できる**（`w_abs` の係数等）。
- **閾値（Decision）**: read は**連続値の advisory** を維持し、「このユーザーは troll」等の**断定閾値は置かない**（trust-semantics §4）。client 表示のバケット化（例 low / mid / high）が要るなら **operator 可変パラメータ**とし、ADR では固定しない。
- **decay（Decision）**: **相対成分・node-local 観測**には **半減期方式の時間減衰**を導入する（半減期は operator 可変）。**絶対成分は evidence / 検知ベースのため減衰させない**（known-hash / provider-verdict は時間で薄めない）。
- **appeal（Decision）**: `AppealStatus` を trust 寄与へ次のとおり反映する。
  - `pending`: 該当 signal の寄与を**据え置き**（申し立て中に勝手に緩めない）。
  - `accepted`: 該当 signal の寄与を**除外**して再計算する。
  - `rejected`: 確定寄与のまま維持する。

### 6.3 visibility / cross-node 開示（pull 型）・opt-out・private channel（Decision）

- **read は pull 型（Decision）**: ある pubkey について read すると、**その CN の視点**で計算された trust / relation が返る。push 複製はしない。`visibility` は **cross-node pull に対するアクセス範囲**（誰がこの CN の signal を read できるか）を表す。
  - `Local`: この CN 自身のクライアントの read にのみ応答（他 node の pull には返さない）。**既定**。
  - `SubscribedNodes`: この CN を subscribe している node からの pull に応答してよい。
  - `Public`: 誰でも pull 可。
- **cross-node pull の開示範囲（Decision）**: cross-node pull に対しては **confirmed 絶対成分のみ**を根拠つき（issuer / basis / confidence / expiry）で応答する。**相対成分・relation・suspected は `Local` 固定**とし、cross-node pull には返さない（誤検知・文化依存指標を拡散しない）。
- **viewer 相対 read の認証（Decision, なりすまし防止）**: 相対成分 / relation は viewer 相対のため read query に **viewer（誰視点か）** を含める必要がある。ユーザー C がユーザー A を騙って「A から見た B の trust / relation」を取得することを防ぐため、**viewer 相対 read はリクエスタが viewer identity への権限を証明（viewer 鍵による署名を検証）できることを必須**とする。**絶対成分 read は viewer 非依存のため viewer 証明を要さない**が、`visibility` scope には従う。
- **opt-out「見えない」の範囲（Decision）**: user は自分を **(a) 他者の discovery / surfacing 非表示**、かつ **(b) 他者から見た relation read にも出さない**ことを選べる（node-local, 可逆, 説明可能）。opt-out は **trust には影響しない**（troll 判定を回避する手段にしない）し、**social graph canonical の削除でもない**。
- **private channel における扱い（Decision）**: private channel（ADR 0024 admission / ADR 0025 §6.3）由来の co-participation は **channel メンバー + その CN の authority に閉じ、`Local` 固定**とする。当該 channel メンバー可視の relation に閉じ、**public relation / trust には混ぜない**。private channel の secret を提示できる権限者の scope を超えて private 由来 signal を露出させない。
