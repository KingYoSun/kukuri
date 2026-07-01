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
- Replicated?: No（node-local）。critical suspected は配布しない（§2.4）。general は risk signal として visibility 規則に従う
- Rebuildable From: VLM provider の再 scan + `route()`。閾値・policy は operator 設定
- Public Replica / Private Replica / Local Only: node-local server（`cn-core` / Postgres）。派生タグは index（node-local）
- Gossip Hint 必要有無: No
- Blob 必要有無: No（**no permanent blob storage**。VLM への入力は一時 fetch / 参照 hint）
- SQLite projection 必要有無: No（server は Postgres）
- 必須 contract:
  - `vlm_provider_is_openai_compatible_and_operator_owned`
  - `vlm_basis_is_classifier_score_never_confirmed`
  - `suspected_threshold_default_0_7_operator_tunable`
  - `high_confidence_critical_is_fail_closed_and_local`
  - `nondeterministic_critical_not_distributed_to_network`
  - `general_moderation_feeds_trust_relative_component`
  - `critical_suspected_feeds_trust_absolute_component`
  - `derived_tags_only_for_allow_media`
  - `derived_tags_exclude_critical_and_match_data`
  - `operator_review_can_edit_detection_metadata`
- 必須 scenario:
  - critical risk タグが閾値超の高 confidence → fail-closed（index されない）かつ visibility `Local`（network 配布しない）
  - general（nsfw / 暴力 / hate）suspected は relation 重み付けで trust 相対成分に入る
  - `allow` media は VLM 派生タグで検索でき、exclude / critical media はタグ化・index されない
  - operator が閾値を 0.7 から変更でき、検知結果メタデータを直接編集（appeal / 誤検知修正）できる

## 1. 背景と意図

- 決定論的 moderation（ADR 0027）は known-hash / provider-verdict による confirmed（絶対・evidence ベース）。一方、未知 CSAM / CSE の suspected や、nsfw / 暴力 / hate 等の general moderation は **確率的分類（VLM / classifier）**でしか判定できない。
- 確率的判定は誤検知を伴うため、決定論的 confirmed と同じ強さで network に拡散させてはならない（visibility は `Local` 寄り）。
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

### 2.4 critical risk 高 confidence → fail-closed かつ Local（network 拡散しない）
- **VLM が critical risk タグ（CSAM / CSE / grooming）を高 confidence（閾値以上）で付けた場合、fail-closed として扱う**: `allow` にしない（index / discovery / recommendation に出さない）。
- かつ **visibility は `Local`**。確率的 critical 検知は誤検知の可能性があるため、confirmed（決定論）と違い **network に配布しない**（`SubscribedNodes` / `Public` にしない）。これが「high-confidence critical を failed として fail-closed・非拡散にする」意図。
- **非対称の要点**: 決定論的 confirmed（ADR 0027, known-hash）は exclude + `SubscribedNodes` 配布し得るが、**非決定論的 critical は高 confidence でも Local 止まり**。誤検知を public advisory として拡散しない安全側の既定。

### 2.5 trust への振り分け（ADR 0026 の絶対 / 相対）
- **critical（CSAM / CSE / grooming）suspected = 厳格非決定論** → ADR 0026 の trust **絶対成分**の入力（relation 非依存、report-bomb 不動）。§2.4 の fail-closed・Local 扱いと整合。
- **general（nsfw / 暴力 / hate / spam 等）= 文化依存** → ADR 0026 の trust **相対成分**の入力（relation で重み付け、viewer / cluster 相対）。
- いずれも断定ラベルではなく根拠つき risk signal（basis = classifier_score, confidence 付き）。

### 2.6 media 検索タグを VLM の副産物として一体設計（ADR 0025 §2.3）
- 同一 VLM scan が (a) moderation verdict / labels と (b) **descriptive な検索タグ**の両方を生成する（二重スキャンしない）。
- **タグを index するのは `allow` verdict の media のみ**（ADR 0025 §2.3）。exclude / hold / quarantine の media はタグ化・index しない。
- critical 検知結果・Match Data（#391）・生スコアの機微をタグや index に流さない（descriptive tag は一般的記述に限定）。
- タグはサムネイル代替表示（読み込み中 / アダルト・暴力的コンテンツの安全用代替、ADR 0025 §2.3）にも使える。

### 2.7 fail 挙動と非対称（まとめ）
- scan failure / provider unavailable / unscanned は ADR 0027 §2.4 どおり fail-closed（`allow` にしない）。
- 高 confidence critical suspected も fail-closed かつ Local（§2.4）。
- general suspected は hold / quarantine（自動、operator レビュー可）で index させない。visibility は `Local` 既定。

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
- 高 confidence critical でも network に配布しない（Local 止まり、誤検知非拡散）。決定論 confirmed とは非対称。
- `allow` 以外は surfacing に出さない（fail-closed、ADR 0027 §2.4 の単一判定点 `is_indexable()`）。
- 派生タグは `allow` media のみ。critical / Match Data をタグ・index に流さない。
- no permanent blob storage。VLM 入力は一時 fetch / 参照 hint。
- operator 編集は node-local advisory の是正であり user canonical を変更しない。

## 6. 未決事項（要設計・レビュー）
- critical fail-closed 用の「高 confidence」閾値を suspected 閾値（0.7）と共通にするか、別のより厳格な値を operator が設定できるようにするか。
- VLM の image / video / text 別 capability 粒度と、OpenAI-compatible API での vision 入力（media_hint = URL / blob 参照）の受け渡し方式。
- タグ語彙（tag vocabulary）の標準化とサムネイル代替表示の client 挙動（ADR 0025 と共同）。
- operator レビューの監査ログ・appeal（`AppealStatus`）反映と、誤検知修正の risk signal への波及。
- general moderation の細分類（nsfw / 暴力 / hate / spam）と relation 相対化の対応（ADR 0026 §6 と共同）。
