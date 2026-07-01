# ADR 0028: Non-Deterministic Moderation (VLM-Assisted Classification)

## Status
Draft

## Date
2026-06-30

## Base Branch
`main`

## Related
- `docs/adr/0027-deterministic-moderation-critical-safety.md`（共通 verdict / fail-closed / signed event / provider abstraction 枠組み。本 ADR はこれに差し込む）
- `docs/adr/0025-community-node-indexing-foundation.md`（§2.3 media 派生タグ = 本 VLM の副産物として一体設計）
- `docs/adr/0026-community-node-trust-relation-foundation.md`（trust 絶対/相対成分。critical=絶対、general=相対）
- `crates/cn-safety/src/{verdict,policy,provider,capability,signal}.rs`（`Basis::ClassifierScore` / classifier capability / `ProviderScanResult.score`）
- `crates/cn-safety-runtime/src/`（`SafetyOrchestrator`）
- Issue: #391（Project Arachnid Shield = 決定論 provider。operator-owned credentials 方式の先例）, #404（indexing 本体）, #406（runtime 結線）, #410 / ADR 0027（決定論 moderation）, #411（本 ADR）

## 位置づけ

community node の moderation のうち **非決定論的 moderation（VLM / classifier ベースの確率的判定）** の設計を固定する。決定論的 moderation（ADR 0027, 既知 hash / provider-verdict = confirmed）と異なり、本 ADR は `Basis::ClassifierScore` による **suspected** 判定を扱う。ADR 0027 の共通枠組み（verdict / fail-closed / signed event / provider abstraction / visibility）に差し込む形で、classifier 検知の詳細・閾値・fail 挙動・trust への振り分け・media タグ一体設計を定義する。

これは greenfield（既存資料なし）であり、確率的分類の fail 挙動・visibility・human-in-the-loop が決定論的 known-match とは異なるため別 ADR とする。

## Feature Data Classification
- Feature 名: non-deterministic (VLM) moderation classification + 派生 media タグ
- Durable / Transient: verdict / classification は transient。生成される signed moderation event / risk signal / 派生タグは durable（event/signal は #405、タグは index = ADR 0025）
- Canonical Source: derived。VLM provider（OpenAI-compatible API）の scan 結果を policy router に通した派生。canonical な「真の分類」は存在しない（確率的・node-local）
- Replicated?: index / classification は node-local。**advisory（signed moderation event / risk signal）は visibility 規則（local / subscribed_nodes / public）に従って network 配布可**。受け手は opt-in trust input として採用（trust-semantics）
- Rebuildable From: VLM provider の再 scan + `route()`。閾値・policy は operator 設定
- Public Replica / Private Replica / Local Only: node-local server（`cn-core` / Postgres）。派生タグは index（node-local）
- Gossip Hint 必要有無: No
- Blob 必要有無: No（**no permanent blob storage**。VLM への入力は一時 fetch / 参照 hint）
- SQLite projection 必要有無: No（server は Postgres）
- 必須 contract:
  - `vlm_provider_is_openai_compatible_and_operator_owned`
  - `vlm_basis_is_classifier_score_never_confirmed`
  - `suspected_threshold_default_0_7_operator_tunable`
  - `high_confidence_critical_is_fail_closed_indexing`
  - `advisory_is_network_distributable_per_visibility`
  - `false_positive_appeal_path_exists`
  - `appeal_cleared_propagates_and_reverts_trust_contribution`
  - `general_moderation_feeds_trust_relative_component`
  - `critical_suspected_feeds_trust_absolute_component`
  - `derived_tags_only_for_allow_media`
  - `derived_tags_exclude_critical_and_match_data`
  - `operator_review_can_edit_detection_metadata`
- 必須 scenario:
  - critical risk タグが閾値超の高 confidence → fail-closed（自 node の index / discovery / recommendation に出ない）。advisory は visibility 規則に従い network 配布可
  - 誤検知は issuer node への異議申し立て → operator が `Cleared` → 配布済み advisory に伝播し trust 寄与が戻る
  - general（nsfw / 暴力 / hate）suspected は relation 重み付けで trust 相対成分に入る
  - `allow` media は VLM 派生タグで検索でき、exclude / critical media はタグ化・index されない
  - operator が閾値を 0.7 から変更でき、検知結果メタデータを直接編集（appeal / 誤検知修正）できる

## 1. 背景と意図

- 決定論的 moderation（ADR 0027）は known-hash / provider-verdict による confirmed（絶対・evidence ベース）。一方、未知 CSAM / CSE の suspected や、nsfw / 暴力 / hate 等の general moderation は **確率的分類（VLM / classifier）**でしか判定できない。
- 確率的判定は誤検知を伴うが、対策は **配布制限ではなく異議申し立て（appeal）経路の整備**とする（§2.8）。advisory（signed moderation event / risk signal）自体は network 配布可であり、決定論 confirmed とは扱いを変えない（ただし confirmed には昇格させず suspected どまり）。
- VLM は moderation verdict と **descriptive な検索タグ**の両方を同一 pipeline で生成できる（ADR 0025 §2.3 の media タグ）。一体設計で二重スキャンを避ける。

## 2. Decision

### 2.1 VLM provider = OpenAI-compatible API、operator-owned
- VLM provider は **OpenAI-compatible API**（chat / vision）であれば self-host / 外部 API を問わない。operator が endpoint + credentials を設定する（#391 の operator-owned credentials 方式に倣う。kukuri 本体は credentials を同梱・共有しない）。
- `cn-safety` の `SafetyProvider` trait 実装として差す。capability は classifier 系（`NovelCsamImageClassifier` / `NovelCsamVideoClassifier` / `CseTextClassifier` / `GroomingTextClassifier` / `GeneralMediaModeration` / `SpamAbuseModeration`）。
- **basis は常に `Basis::ClassifierScore`。confirmed（`KnownHashMatch` / `ProviderVerdict`）に昇格させない**（ADR 0027 §2.2 と整合）。`ProviderScanResult.score`（0-100）に確信度を載せる。
- 入力は最小参照（`media_hint` / `text`）。blob は scan のため一時 fetch のみ、恒久保存しない。

### 2.2 suspected 閾値 = 既定 0.7、operator 可変
- suspected 判定の既定閾値は **0.7**（score 0-100 換算で 70）。**operator が調整できる**（`suspected_threshold`）。
- 閾値以上を suspected として route する。critical route と general route は ADR 0027 §2.3 の分離に従う。

### 2.3 標準は自動 hold/quarantine、optional で operator レビュー
- **標準挙動は自動 hold / quarantine**（suspected は index させない）。
- **optional**: operator レビューを有効化できる。operator は **検知結果（= メタデータ）を直接編集**できる（誤検知の是正 / 分類の修正）。これは `AppealStatus`（None / Disputed / Cleared）と operator audit に接続する。
- operator 編集は node-local な advisory の是正であり、user の canonical state を変更しない。

### 2.4 critical risk 高 confidence → fail-closed（自 node の index / surface のみ制御）
- **VLM が critical risk タグ（CSAM / CSE / grooming）を高 confidence（閾値以上）で付けた場合、fail-closed として扱う**: `allow` にしない（自 node の index / discovery / recommendation に出さない）。
- ここで制御するのは **その node 自身の surfacing 出力だけ**である。コンテンツ（gossip hint / docs / blob）の P2P 流通を止めるものではない（node の authority scope 外。P2P 上に中央権者はいない）。
- **advisory（signed moderation event / risk signal）は network 配布可**。visibility 規則（`local` / `subscribed_nodes` / `public`）に従って配布でき、受け手は opt-in の trust input として採用する（trust-semantics, ADR 0027 §2.1）。**非決定論だからといって Local に固定しない**。
- 確率的判定の誤検知は **配布制限ではなく異議申し立て（appeal）経路で是正する**（§2.8）。

### 2.5 trust への振り分け（ADR 0026 の絶対 / 相対）
- **critical（CSAM / CSE / grooming）suspected = 厳格非決定論** → ADR 0026 の trust **絶対成分**の入力（relation 非依存、report-bomb 不動）。§2.4 の fail-closed 扱いと整合。
- **general（nsfw / 暴力 / hate / spam 等）= 文化依存** → ADR 0026 の trust **相対成分**の入力（relation で重み付け、viewer / cluster 相対）。
- いずれも断定ラベルではなく根拠つき risk signal（basis = classifier_score, confidence 付き）。

### 2.6 media 検索タグを VLM の副産物として一体設計（ADR 0025 §2.3）
- 同一 VLM scan が (a) moderation verdict / labels と (b) **descriptive な検索タグ**の両方を生成する（二重スキャンしない）。
- **タグを index するのは `allow` verdict の media のみ**（ADR 0025 §2.3）。exclude / hold / quarantine の media はタグ化・index しない。
- critical 検知結果・Match Data（#391）・生スコアの機微をタグや index に流さない（descriptive tag は一般的記述に限定）。
- タグはサムネイル代替表示（読み込み中 / アダルト・暴力的コンテンツの安全用代替、ADR 0025 §2.3）にも使える。

### 2.7 fail 挙動（まとめ）
- scan failure / provider unavailable / unscanned は ADR 0027 §2.4 どおり fail-closed（`allow` にしない）。
- 高 confidence critical suspected も fail-closed（`allow` にしない、§2.4）。
- general suspected は hold / quarantine（自動、operator レビュー可）で index させない。
- fail-closed は **自 node の surfacing 制御**であり、advisory の配布可否とは独立（advisory は §2.4 のとおり network 配布可）。visibility の既定は安全側だが hard cap ではなく policy / operator で調整でき、誤検知は §2.8 の appeal で是正する。

### 2.8 誤検知への異議申し立て（appeal）経路
確率的判定は誤検知を伴うため、**配布を制限するのではなく、誤検知を是正できる異議申し立て経路を整備する**ことを安全策の中心に置く。
- **状態**: `AppealStatus`（`None` / `Disputed` / `Cleared`）で risk signal / moderation event の異議状態を管理する。
- **申し立て導線**: user / client は、その advisory を発行した **issuer node**（責任 node）へ異議を申し立てられる。分散通報ルーティングが issuer node の abuse / appeal endpoint を候補化する（ADR 0027 §2.8, report routing）。
- **operator レビュー**: operator は検知メタデータを直接編集して `Disputed` → `Cleared` にできる（§2.3）。
- **是正の伝播**: 既に配布した advisory は、`Cleared` 反映 / `expires_at` 失効 / 訂正 signal の再発行で受け手に伝える。受け手は opt-in trust input として最新状態を反映する。
- **trust への反映戻し**: `Cleared` になった誤検知は、ADR 0026 の trust 絶対 / 相対成分への負の寄与を取り消す。

## 3. Consequences
- 非決定論 moderation の decision record が確定し、ADR 0027（決定論）と対で moderation 設計が揃う。
- VLM provider（OpenAI-compatible）を `SafetyProvider` 実装として追加する実装 Issue が必要（basis=classifier_score, 閾値 0.7 可変, operator review, タグ生成）。
- media 検索タグ（ADR 0025）と moderation を一体の VLM pipeline として実装する（ADR 0025 §2.3 のタグ生成は本 ADR の VLM が供給）。
- trust（ADR 0026）の絶対成分（critical suspected）/ 相対成分（general）への入力経路が明確化される。

## 4. Out of scope（後続 / 別 Issue）
- VLM provider 実装（OpenAI-compatible client / capability マッピング / タグ生成）。
- 具体的 prompt / モデル選定 / タグ語彙（tag vocabulary）の標準化。
- operator レビュー UI（admin 画面, #382）。
- trust scoring の合成詳細（ADR 0026 §6）。

## 5. 維持する境界
- 非決定論 = 常に `Basis::ClassifierScore` = suspected。confirmed に昇格させない。
- fail-closed は自 node の surfacing 制御であり、advisory の network 配布可否とは独立。非決定論だからといって advisory を Local に固定しない。誤検知は appeal 経路で是正する（§2.8）。
- `allow` 以外は surfacing に出さない（fail-closed、ADR 0027 §2.4 の単一判定点 `is_indexable()`）。
- 派生タグは `allow` media のみ。critical / Match Data をタグ・index に流さない。
- no permanent blob storage。VLM 入力は一時 fetch / 参照 hint。
- operator 編集は node-local advisory の是正であり user canonical を変更しない。

## 6. 未決事項（要設計・レビュー）
- critical fail-closed 用の「高 confidence」閾値を suspected 閾値（0.7）と共通にするか、別のより厳格な値を operator が設定できるようにするか。
- VLM の image / video / text 別 capability 粒度と、OpenAI-compatible API での vision 入力（media_hint = URL / blob 参照）の受け渡し方式。
- タグ語彙（tag vocabulary）の標準化とサムネイル代替表示の client 挙動（ADR 0025 と共同）。
- appeal 経路の詳細（申し立て API / issuer の appeal endpoint / 配布済み advisory への `Cleared` 伝播・失効の具体・監査ログ）。
- general moderation の細分類（nsfw / 暴力 / hate / spam）と relation 相対化の対応（ADR 0026 §6 と共同）。
