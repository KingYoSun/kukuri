# ADR 0028: Non-Deterministic Moderation (VLM-Assisted Classification)

## Status
Accepted（#420 で実装。§6 の実装済み決定は「§7 実装追補」を参照）

**2026-09-15 改訂**（#1051）: general moderation のうち **nsfw と新設 `objectionable` の suspected は
index から除外せず、`allow` + content advisory ラベルとして index・配信**し、**trust への寄与は 0**
とする。guard 写像・operator knob・client への advisory 配信・text / media の合成を含む決定は
「§8 改訂追補」を正本とし、§2.3 / §2.5 / §2.7 / §7.2 の該当記述は失効注記で後継を示す。
critical route（CSAM / CSE / grooming）の fail-closed、spam / malware / phishing の `Exclude`、
`Basis::ClassifierScore` の不昇格、no permanent blob storage は変更しない。

## Date
2026-06-30（実装追補: 2026-07-30、改訂: 2026-09-15、追補: 2026-09-17）

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
  - `general_moderation_feeds_trust_relative_component`（2026-09-15 superseded: 対象を spam / malware / phishing に限定し `spam_malware_phishing_feed_trust_relative_component` へ改名。nsfw / objectionable は `general_advisory_contributes_zero_to_trust`。§8.5）
  - `critical_suspected_feeds_trust_absolute_component`
  - `derived_tags_only_for_allow_media`（label 付き `allow` も `allow` であり対象に含む。§8.8）
  - `derived_tags_exclude_critical_and_match_data`
  - `operator_review_can_edit_detection_metadata`
  - 2026-09-15 追加（#1051、§8.11）:
    - `general_nsfw_is_indexed_with_advisory_label`
    - `general_advisory_contributes_zero_to_trust`
    - `objectionable_category_separated_from_nsfw`
    - `content_advisories_are_separate_from_signed_content_labels`
    - `general_action_operator_tunable_stricter_only`
    - `labeled_allow_emits_risk_label_event_and_signal`
    - `labeled_allow_text_does_not_short_circuit_media_scan`
  - 2026-09-17 追加（#1109、§8.14）: `rescan_allow_without_labels_expires_stale_advisory_signal`、
    `rescan_does_not_expire_protected_signals`、`expire_superseded_advisory_signals_migration`
- 必須 scenario:
  - critical risk タグが閾値超の高 confidence → fail-closed（自 node の index / discovery / recommendation に出ない）。advisory は visibility 規則に従い network 配布可
  - 誤検知は issuer node への異議申し立て → operator が `Cleared` → 配布済み advisory に伝播し trust 寄与が戻る
  - general（nsfw / 暴力 / hate）suspected は relation 重み付けで trust 相対成分に入る（2026-09-15 superseded: nsfw / objectionable は寄与 0 で basis にのみ残る。spam / malware / phishing は従来どおり相対成分。§8.5）
  - `allow` media は VLM 派生タグで検索でき、exclude / critical media はタグ化・index されない（label 付き `allow` media もタグ化される。§8.8）
  - operator が閾値を 0.7 から変更でき、検知結果メタデータを直接編集（appeal / 誤検知修正）できる
  - 2026-09-15 追加（#1051）: nsfw suspected の画像付き投稿は index され、検索 / 発見 / おすすめの結果に `content_advisories`（`label = adult`）が付き、投稿者の trust read の basis に寄与 0 で現れる。相対成分は動かない
  - 2026-09-15 追加（#1051）: guard mode の B / C / G 検知は `objectionable`（`label = sensitive`）として同じ経路を通り、A のみが nsfw になる
  - 2026-09-15 追加（#1051）: `general_action = exclude` の node では nsfw / objectionable は従来どおり index されず、`allow`（ラベル無し）は設定として受理されない

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

> **2026-09-15 改訂**（#1051、§8.1）: 本節の「標準挙動は自動 hold / quarantine（suspected は index させない）」は
> **critical suspected（未知 CSAM / CSE / grooming）にのみ**適用する。nsfw / objectionable の suspected は
> 標準で `allow` + content advisory ラベルとして index し、hold / exclude は operator が `general_action` で
> 厳格化した場合に限る。spam / malware / phishing は `Exclude` のまま。

### 2.4 critical risk 高 confidence → fail-closed（自 node の index / surface のみ制御）
- **VLM が critical risk タグ（CSAM / CSE / grooming）を高 confidence（閾値以上）で付けた場合、fail-closed として扱う**: `allow` にしない（自 node の index / discovery / recommendation に出さない）。
- ここで制御するのは **その node 自身の surfacing 出力だけ**である。コンテンツ（gossip hint / docs / blob）の P2P 流通を止めるものではない（node の authority scope 外。P2P 上に中央権者はいない）。
- **advisory（signed moderation event / risk signal）は network 配布可**。visibility 規則（`local` / `subscribed_nodes` / `public`）に従って配布でき、受け手は opt-in の trust input として採用する（trust-semantics, ADR 0027 §2.1）。**非決定論だからといって Local に固定しない**。
- 確率的判定の誤検知は **配布制限ではなく異議申し立て（appeal）経路で是正する**（§2.8）。

### 2.5 trust への振り分け（ADR 0026 の絶対 / 相対）
- **critical（CSAM / CSE / grooming）suspected = 厳格非決定論** → ADR 0026 の trust **絶対成分**の入力（relation 非依存、report-bomb 不動）。§2.4 の fail-closed 扱いと整合。
- **general（nsfw / 暴力 / hate / spam 等）= 文化依存** → ADR 0026 の trust **相対成分**の入力（relation で重み付け、viewer / cluster 相対）。
- いずれも断定ラベルではなく根拠つき risk signal（basis = classifier_score, confidence 付き）。

> **2026-09-15 改訂**（#1051、§8.5）: general のうち **nsfw / objectionable は trust 相対成分へ寄与しない（0）**。
> risk signal は生成・永続化し、利用者向け trust read の basis に寄与 0 で残す（説明・appeal 用）。
> 相対成分の入力として残る general は spam / malware / phishing と node-local 観測である。

### 2.6 media 検索タグを VLM の副産物として一体設計（ADR 0025 §2.3）
- 同一 VLM scan が (a) moderation verdict / labels と (b) **descriptive な検索タグ**の両方を生成する（二重スキャンしない）。
- **タグを index するのは `allow` verdict の media のみ**（ADR 0025 §2.3）。exclude / hold / quarantine の media はタグ化・index しない。
- critical 検知結果・Match Data（#391）・生スコアの機微をタグや index に流さない（descriptive tag は一般的記述に限定）。
- タグはサムネイル代替表示（読み込み中 / アダルト・暴力的コンテンツの安全用代替、ADR 0025 §2.3）にも使える。

### 2.7 fail 挙動（まとめ）
- scan failure / provider unavailable / unscanned は ADR 0027 §2.4 どおり fail-closed（`allow` にしない）。
- 高 confidence critical suspected も fail-closed（`allow` にしない、§2.4）。
- general suspected は hold / quarantine（自動、operator レビュー可）で index させない。
  （2026-09-15 superseded: nsfw / objectionable は `allow` + content advisory で index する。§8.1。spam / malware / phishing は `Exclude` のまま）
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
- ~~critical fail-closed 用の「高 confidence」閾値を suspected 閾値（0.7）と共通にするか、別のより厳格な値を operator が設定できるようにするか。~~ → **#420 で「共通 1 本」に決定**（§7.1）。
- ~~VLM の image / video / text 別 capability 粒度と、OpenAI-compatible API での vision 入力（media_hint = URL / blob 参照）の受け渡し方式。~~ → **#420 で決定**: 入力は `media_hint` から `MediaFetcher` で一時 fetch した bytes を data URL（`image_url`）として渡す。capability は検知 category と media content type から導く（video/* → `NovelCsamVideoClassifier`）。§7.2。
- タグ語彙（tag vocabulary）の標準化とサムネイル代替表示の client 挙動（ADR 0025 と共同）。
- ~~appeal 経路の詳細~~ → **#420 で決定**: 申し立ては既存 `POST /v1/report` の optional `appeal.risk_signal_id` で受理（専用 endpoint / manifest の appeal_endpoint は新設しない。report routing の既存候補化を再利用）。`Cleared` は失効させず配布に残して伝播し、受け手の trust 供給層が除外する。訂正は「訂正 signal 再発行 + 旧 signal の終結」（審査経路は #710 で旧 signal を `Cleared` として終結させる。`cn-cli` の appeal を伴わない個別再発行は従来どおり旧 signal へ `expires_at`）。§7.3。監査ログは後続。
- ~~general moderation の細分類（nsfw / 暴力 / hate / spam）と relation 相対化の対応（ADR 0026 §6 と共同）。~~ → **#1051 で決定**（§8.2 / §8.5）: nsfw と objectionable を分け、いずれも trust 寄与 0 の content advisory として扱う。relation 相対化の対象は spam / malware / phishing と node-local 観測に限る。

## 7. 実装追補（#420、2026-07-30）

実装 crate は `crates/cn-safety-vlm`（provider 名 `openai-compatible-vlm`、`general` /
`unknown_csam` slot 専用。`known_csam` slot への指定は fail-closed で拒否）。

### 7.1 閾値
`SafetyPolicy.unknown_csam_score_threshold` を `suspected_threshold` に改名（旧名は serde alias
で受理）し、既定を 70（= 0.7）に変更。critical fail-closed 用の閾値は**分けない**（共通 1 本）。
理由: router 規則 6 が閾値未満の critical 検知を既に Hold へ fail-closed しており、閾値は
「Quarantine か Hold か」の境界にすぎず、critical が Allow に落ちる経路は閾値に依存しない。
general route も同じ閾値に従う（score / confidence を持つ検知のみ gating。categorical 検知は
従来どおり発火）。operator は `COMMUNITY_NODE_SAFETY_SUSPECTED_THRESHOLD`（1-100）で可変。

### 7.2 provider 実装
- OpenAI-compatible `POST {base}/v1/chat/completions`。応答解釈は 2 モード
  （`COMMUNITY_NODE_VLM_RESPONSE_FORMAT`）:
  - `json`（既定）: system prompt で厳密 JSON（categories + tags）を要求する汎用モード。
    未知カテゴリは `Protocol` → fail-closed。
  - `guard`: guard 系モデル（SingGuard 等の「1 行目 safe/unsafe + `<answer>` カテゴリ」形式）
    向け。score は先頭トークンの logprob（`exp(logprob)` を 0-100 換算。logprobs 欠落時は
    保守的に 100）。guard の粗いカテゴリからは **critical へ写像しない**（A→nsfw、D→phishing、
    E→spam、B/C/G→nsfw、F=政治的内容は moderation 対象にしない）。
    （2026-09-15 superseded: B / C / G は `objectionable` へ写像する。A→nsfw、D→phishing、E→spam、F=非検知は不変。§8.2）
- API key は optional（self-host の無認証 endpoint を許容）。endpoint / model は既定値を持たず
  operator が必ず指定する。credentials の値は Debug / エラーに出さない。
- `known_hash_match` を設定する経路が構造的に無い = basis は常に `ClassifierScore`。
- critical を検知した scan では `derived_tags` を空にする（収集側
  `derived_tags_for_index` の allow-only / critical 除外と合わせた二重防御）。
- `MediaFetcher` の本番実装は #609 で追加（`cn-indexer` の `BlobMediaFetcher`）。取得は
  iroh-blobs の **store 非経由**転送（`get::request::get_blob` → memory 直接）で行い、remote
  から取得した bytes をローカルストアへ書かない = no-permanent-blob-storage を構造的に守る
  （スキャン後の破棄処理が不要）。サイズ上限 / timeout は
  `COMMUNITY_NODE_MEDIA_FETCH_MAX_BYTES`（既定 32 MiB）/
  `COMMUNITY_NODE_MEDIA_FETCH_TIMEOUT_SECS`（既定 30 秒）で operator が可変。取得不能は
  `Unavailable`、サイズ超過・content type 不明は `Protocol`、時間切れは `Timeout`
  （いずれも fail-closed）。content type は参照元 metadata（`AssetRef.mime` / manifest item）を
  scan request の `media_mime` で運び、欠落時は magic bytes 判定に fallback する。
  manifest 参照は cn-indexer の ingest が replica 上の署名済み manifest（post author 本人の署名を
  要求）を item blob（hash + mime）へ展開してから scan する。未構成なら media scan
  は `Unavailable` → fail-closed。

### 7.3 visibility / appeal
- suspected advisory の visibility は `SafetyPolicy.suspected_signal_visibility`（既定 `Local`、
  operator が `SubscribedNodes` / `Public` へ可変 = §2.4 の「Local に固定しない」）。
  operational fail-closed（content category 無し）は常に `Local`。
- appeal 遷移は `None → Disputed`（申し立て）/ `Disputed → Cleared`（認容）/
  `Disputed → None`（棄却）のみ。operator レビュー（メタデータ編集 / 訂正再発行）は
  `safety.moderation.operator_review`（env `COMMUNITY_NODE_SAFETY_OPERATOR_REVIEW`）の
  明示的有効化が必要。運用は `cn-cli moderation` コマンド。
- `Cleared` はノード内の利用者向け信頼評価取得に実効寄与 0 の `basis` として残し、利用者が
  審査結果を再取得できるようにする。評価計算と別ノード向け取得からは除外する。
- **訂正版再発行の終結（#710・案A）**: 審査経路の再発行は、旧 signal を失効させる代わりに
  **同一取引で `Cleared` にして終結**させ、関連通報を処理済み（actioned）にする。失効させると
  trust 供給層の失効除外で根拠一覧から消え、利用者が審査の終結を確認できないため。旧 signal は
  `Cleared` の既存伝播契約（失効させず配布に残し、受け手が寄与 0 で保持）に乗り、訂正版の
  新 signal と二重寄与しない。appeal を伴わない `cn-cli` の個別再発行（運用是正）は従来どおり
  旧 signal へ `expires_at` を刻む。contract:
  `post_appeal_reissue_closure_is_visible_after_trust_refetch`。
- **operator 確定値の保護（#1058、2026-09-16）**: 審査・`cn-cli` の検知メタデータ編集と訂正版再発行で
  値を確定した risk signal は、operator 確定の印（`operator_adjusted_at`）と最初の訂正前の category
  （`operator_origin_category`）を持つ。scan 構成の変更などによる再 scan は、同じ issuer / target /
  basis で category か訂正前の category が一致する印付きの行があれば、失効・`Cleared` を問わず
  値を更新せず新しい行も作らない（#1050 の集約更新より優先する）。別 category の新しい判定は抑止しない。
  訂正版は scanner の集約経路を通さず印付きで挿入するため、訂正済みの判定も再発行できる。
  棄却（`Disputed → None`）は印を付けない。印は利用者向け信頼評価取得の `basis`
  （`operator_adjusted_at`）と `cn-cli moderation show` / `list-signals` で判別できる。印を外して
  scanner の判定へ戻す操作は持たない。migration は審査経路の過去操作を操作記録から復元し、記録の無い
  `cn-cli` の過去操作は復元しない。contract: `operator_adjusted_signal_survives_rescan`、
  `appeal_review_adjustments_survive_rescan`、`trust_read_basis_marks_operator_adjusted_signals`。
- 署名済み moderation event は不変（是正は risk signal 側）。

## 8. 改訂追補（#1051、2026-09-15）: general 判定の index + content advisory 化

本節は 2026-09-15 の製品決定（Issue #1051）を記録する。§2.3 / §2.5 / §2.7 / §7.2 の失効注記は
本節を後継とする。実装は child Issue（CN 側 = C2、client 見つける = C3、タイムライン合成 = C4）が
担い、本節の contract 名を test 名として使う。

### 8.1 verdict と index

- nsfw / objectionable（§8.2）の suspected（score ≥ `suspected_threshold`）は **`SafetyAction::Allow` に
  `SafetyVerdict.advisory_labels`（category / confidence / provider_capability）を伴う verdict** とし、
  index する。新しい action は追加しない。`SafetyAction::allows_indexing` / `SafetyVerdict::is_indexable()` が
  `Allow` のみを indexable とする単一判定点、`cn_index.index_entries` の `CHECK (verdict_action = 'allow')` /
  `CHECK (NOT critical)`（ADR 0025 §6.7）はいずれも不変。
- spam / malware / phishing は従来どおり `Exclude`。critical route（§2.4、ADR 0027 §2.3）は不変で、
  閾値未満でも `Allow` に落ちない取りこぼし防止（規則 6）も変えない。
- `reason_code` は `GeneralModeration` のままとし、`basis_for_verdict` は `Basis::ClassifierScore` を返す
  （confirmed へ昇格しない）。`policy_version` は `2026-09-public-node-v3` へ更新する。

### 8.2 category と guard / json 写像

- `SafetyCategory::Objectionable` を新設する（general route、`is_critical_safety() = false`）。real-world
  crimes / unethical・hate・harassment / animal abuse の傘であり、nsfw（性的表現）と分ける。
- guard mode の写像は **A → `Nsfw`、B / C / G → `Objectionable`、D → `Phishing`、E → `Spam`、F → 非検知**
  とする（§7.2 の「B/C/G→nsfw」を失効）。critical へは引き続き写像しない。
- json mode のカテゴリ列挙に `objectionable` を追加する。未知カテゴリは従来どおり `Protocol` → fail-closed。
- objectionable の index / trust / 配信の扱いは nsfw と同一（label のみ `sensitive`）。

### 8.3 text と media の合成

- post 本文 text の verdict が label 付き `allow` の場合、**media scan を短絡せずに実行**する（従来は
  非 allow で短絡していたが、label 付き allow は indexable であるため到達する）。
- post の index 可否は text と各 blob の verdict の worst-case 合成（1 つでも非 allow なら index しない）。
- post の `content_advisories` は text と各 blob の advisory の和集合とし、blob 単位の advisory は
  `subject_kind = blob_cid` で保持する（ADR 0046 §4 の hash 単位取得ゲートに使う）。

### 8.4 artifact（signed moderation event / risk signal）

- label 付き `allow` でも signed moderation event（`ModerationAction::RiskLabel`）と risk signal を
  生成・永続化する。severity は `Low`、basis は `ClassifierScore`、visibility は
  `suspected_signal_visibility` に従う。§2.8 の appeal 経路（`None → Disputed → Cleared | None`）の対象。
- signed moderation event は不変、是正は risk signal 側（§7.3）という規則は変わらない。
- 同一 subject の再 scan で signal / event を重複永続化しない扱いは #1050 が所有する。

### 8.5 trust への寄与

- nsfw / objectionable の risk signal は **trust の評価計算に入れない（寄与 0）**。`trust_component_for` の
  分岐は critical = 絶対 / spam・malware・phishing・node-local 観測 = 相対 / nsfw・objectionable =
  advisory-only（0）の 3 分岐になる（ADR 0026 追補 §7）。
- 利用者向け trust read（`GET /v1/trust/users/{pubkey}`）の basis には `Cleared` と同様に
  `raw_contribution = 0` / `contribution = 0` で残し、利用者が判定と appeal 状態を確認できるようにする。
  評価値・別ノード向け pull（ADR 0026 §6.3）には入れない。

### 8.6 wire: `content_advisories`

- index entry（`IndexEntryView`）と §8.12 の一括照会応答に、署名済み `content_labels` とは**別欄**の
  `content_advisories` を載せる。要素は次を持つ:
  `issuer_node_id`（署名鍵の x-only 公開鍵 hex = manifest `node_id`、ADR 0027 §2.5）、
  `subject_kind`（`post_id` | `blob_cid`）、`subject_id`、`category`（`SafetyCategory`）、
  `label`（client 表示語彙。nsfw → `adult`、objectionable → `sensitive`）、`confidence`（0-100）、
  `signal_id`、`basis`（常に `classifier_score`）。
- `content_advisories` は投稿の canonical でも署名対象でもない node-local な advisory であり、client は
  署名済み投稿を解決した後も第 2 のラベル源として保持する（ADR 0046 追補 §6）。`content_labels` へ
  書き戻さない（`content_advisories_are_separate_from_signed_content_labels`）。
- `IndexEntryView.text` と同じく、`content_advisories` を canonical content の信頼元にしない。

### 8.7 operator knob

- `safety.moderation.general_action`（env `COMMUNITY_NODE_SAFETY_GENERAL_ACTION`）= `label`（既定）|
  `hold` | `exclude`。nsfw / objectionable の suspected に適用する。`allow`（ラベル無し）は型に variant を
  持たせず受理しない（`general_action_operator_tunable_stricter_only`）。旧 `on_high_confidence_nsfw` の
  `hold` / `exclude` は serde alias で受理し、`allow` は拒否する。
- operator config の `SafetyProviderEntry.on_high_confidence` は宣言のみで実効性が無かったため
  deprecated とし、読み捨てて警告する。`validate-config` は `general_action` の値域を検証する。
- 既存の `suspected_threshold` / `suspected_signal_visibility` / `operator_review` は不変。

### 8.8 派生タグ

- label 付き `allow` の media もタグ化・index する（§2.6 の「`allow` verdict の media のみ」は文言どおり
  有効）。critical 検知・Match Data・生スコアをタグに流さない規則は不変。
- タグのサムネイル代替表示（ADR 0025 §2.3）の一次判定は `content_advisories` に移り、タグは補助情報。

### 8.9 backfill

- 過去に `Exclude` された nsfw 投稿は、`policy_version` 更新に伴う再 scan / 再 ingest で自然に index へ
  入る。専用の backfill migration は行わない。#1050 の verdict 再利用は鍵に `policy_version` を含める
  ため、v3 への更新が再 scan の契機になる。

### 8.10 Feature Data Classification（content advisory の client 配信）

- Feature 名: community node content advisory（nsfw / objectionable の suspected ラベル）の client 配信
- Durable / Transient: node 側は durable（`cn_safety.scan_verdicts.advisory_labels` + risk signal）。client 側は
  transient（表示 state と取得ゲート用 hash 集合のみ。永続化しない）
- Canonical Source: 存在しない（issuer node の node-local 判定）。投稿の canonical は author-owned のまま
- Replicated?: No。node の client が API で pull する（index entry 同梱、または §8.12 の一括照会）
- Rebuildable From: 再取得（node 側は再 scan）
- Public Replica / Private Replica / Local Only: node-local。配信範囲は `Local`（自 node のクライアント）。
  cross-node pull（ADR 0026 §6.3）には含めない
- Gossip Hint 必要有無: No
- Blob 必要有無: No（advisory 付き media も no permanent blob storage。client 側取得は ADR 0046 §4 のゲート）
- SQLite projection 必要有無: client は advisory 付き blob hash の一時集合を取得ゲート判定に使う
  （永続 projection にするか in-memory にするかは C3 / C4 で決定し ADR 0046 の分類文書へ記録）

### 8.11 contract / scenario 更新

| contract | 状態 | 配置予定 |
| --- | --- | --- |
| `general_nsfw_is_indexed_with_advisory_label` | 追加 | cn-safety `policy_router.rs` / cn-indexer `ingestion_contracts.rs` |
| `general_advisory_contributes_zero_to_trust` | 追加（ADR 0026 と共有） | cn-trust / cn-core `trust_inputs.rs` |
| `objectionable_category_separated_from_nsfw` | 追加 | cn-safety-vlm `provider_contract.rs` / cn-safety |
| `content_advisories_are_separate_from_signed_content_labels` | 追加 | cn-user-api `indexing` contract / desktop vitest |
| `general_action_operator_tunable_stricter_only` | 追加 | cn-safety `policy_router.rs` / cn-operator config test |
| `labeled_allow_emits_risk_label_event_and_signal` | 追加 | cn-safety-runtime `orchestrator.rs` |
| `labeled_allow_text_does_not_short_circuit_media_scan` | 追加 | cn-indexer `ingestion_contracts.rs` |
| `general_moderation_feeds_trust_relative_component` | superseded → `spam_malware_phishing_feed_trust_relative_component` | cn-core `trust_inputs.rs` |
| `high_confidence_critical_is_fail_closed_indexing` / `derived_tags_only_for_allow_media` / `derived_tags_exclude_critical_and_match_data` / `vlm_basis_is_classifier_score_never_confirmed` | 維持 | 従来どおり |

scenario は Feature Data Classification の「2026-09-15 追加」3 件を正本とする。

### 8.12 実装で同期する箇所（child への申し送り）

| 箇所 | 内容 | 担当 |
| --- | --- | --- |
| `crates/cn-safety/src/policy.rs` / `verdict.rs` | `advisory_labels`、`general_action` 型、`Objectionable`、`policy_version` v3 | C2 |
| `crates/cn-safety-vlm/src/client.rs` / `provider.rs` | guard 写像 A→nsfw / B・C・G→objectionable、json `objectionable` | C2 |
| `crates/cn-safety-runtime/src/artifacts.rs` | label 付き allow で `RiskLabel` event + `Low` signal | C2 |
| `crates/cn-trust/src/score.rs` / `inputs.rs`、`crates/cn-core/src/trust_inputs.rs` | nsfw / objectionable の寄与 0、basis 残置、pull 除外 | C2 |
| `crates/cn-core/migrations` | `cn_safety.scan_verdicts.advisory_labels`（JSONB、既定 `[]`） | C2 |
| `crates/cn-protocol/src/index.rs`、`apps/desktop/src/lib/api/types.generated.ts` | `IndexEntryView.content_advisories` | C2 |
| `crates/cn-operator/src/safety_config.rs` / `docs.rs` / `lib.rs` | `general_action`、`on_high_confidence` deprecated、`gen_moderation_policy` 文言 + 文書 version 2 | C2 |
| `infra/terraform/modules/gcp-vm-compose/variables.tf` / `templates/community-node.env.tftpl`、`docker-compose.community-node.yml` | `COMMUNITY_NODE_SAFETY_GENERAL_ACTION` passthrough | C2 |
| `apps/desktop/src/components/core/CommunityIndexWorkspace.tsx` / `communityIndexPostCardView.ts`、`crates/app-api/src/media.rs` | 見つけるの advisory ゲートと取得ゲート登録 | C3 |
| cn-user-api 新 route（一括照会）、desktop タイムライン合成、`docs/legal/external-transmission-notice.md` / `docs/legal/app-data-flow-inventory.md` | タイムライン向け advisory 照会 | C4 |
| `docs/legal/terms-of-service.md` 第3条 4 項、`LEGAL_BUNDLE_VERSION`、i18n `legal` namespace | 第 2 ラベル源の明記と再同意 | C4 |

一括照会 API（仮名 `POST /v1/advisories/lookup`）は認証 + 同意済み client から可視 post id / blob hash の
集合を受け、**その node 自身が発行した** advisory のみ返す。`Cleared` と失効は除外し、index scope 状態を
変更せず、相対 trust / relation を返さない（`advisory_lookup_returns_only_configured_node_signals`）。

### 8.13 変更しないもの

- critical route の fail-closed、`require_known_csam`、unscanned / scan_failed / provider_unavailable の
  非 index、`is_indexable()` 単一判定点、DB CHECK 制約。
- `Basis::ClassifierScore` は confirmed に昇格しない。cross-node pull は confirmed 絶対成分のみ。
- no permanent blob storage。VLM 入力は一時 fetch。
- ラベル無し（self-label も advisory も無い）投稿は通常表示（ADR 0046 の fail-open の限界は不変）。

### 8.14 再 scan 後の advisory signal の整合（#1109、2026-09-17）

一括照会（risk signal 由来）と index read の `content_advisories`（verdict 行由来）は、同じ subject について
同じ現在の判定を返す。provider / policy 構成の変更後に旧構成の signal が残り、照会だけが advisory を返す
食い違いを次の規則で解消する。

- provider を呼んだ再 scan（別 subject の内容 cache 再利用を含む）が index 可能な verdict を返したら、それを
  subject の現在の判定とする。同じ issuer・target・target_id の nsfw / objectionable signal のうち、その
  verdict の `advisory_labels` に無い category の行へ、新しい判定の `scanned_at` を `expires_at` として刻む。
  残る category の行は #1050 の集約更新に従う。
- 構成変更による再 scan と同一構成の再 scan は区別しない。同一構成・同一内容は保存済み verdict の再利用で
  provider を呼ばず、signal にも触れない。
- hold / exclude / 失敗の再 scan では失効させない（subject は index されず、signal は従来どおり集約される）。
- 失効させない行: operator 確定の印がある行（§7.3、#1058）、`appeal_status` が `Disputed` / `Cleared` の行、
  appeal 通報から参照される行（棄却 = 判定維持を含む）、critical・spam / malware / phishing、
  `ClassifierScore` 以外の basis。operator・審査の判断を scanner の判定より優先するため、nsfw / objectionable の
  `Disputed` 行・operator 確定行・棄却済み行が残る subject では照会が advisory を返し続ける（`Cleared` 行は
  従来どおり照会から除外される）。
- 行は削除しない。signed moderation event は不変で追加発行もしない（§7.3）。配布済み advisory は既存の
  `expires_at` 失効契約（配布クエリと trust 供給から除外）で伝わる。nsfw / objectionable は元から trust 寄与 0
  のため評価値は変わらず、利用者向け trust read の basis から失効行が外れる。
- 一括照会は verdict を join しない。signal を真実源とする読み口（`Cleared` と失効の除外）を保ち、書き込み側で
  signal を現在の判定へ揃える。
- 規則より前に再 scan 済みの subject は、migration `202609170003_expire_superseded_advisory_signals.sql` が
  同じ規則で揃える（verdict 行が `allow` で自 subject の advisory に同じ category が無く、signal が verdict 行の
  最終更新以前に保存された行）。
- post 行に同梱した参照 blob の advisory は、その post の再 ingest で再計算する（従来どおり）。
- contract: `rescan_allow_without_labels_expires_stale_advisory_signal`、
  `rescan_keeps_only_current_advisory_categories`、`rescan_does_not_expire_protected_signals`、
  `non_indexable_rescan_keeps_advisory_signals`、`reused_verdict_does_not_touch_signals`、
  `expire_superseded_advisory_signals_migration`。
- 判断・inventory・検証は [作業記録](../progress/2026-09-17-1109-advisory-rescan-consistency.md) を参照する。

## 9. OpenAI Moderation・動画抽出・内容hash再利用（#1060）

2026-09-17承認、Scope revision `2026-09-17-expanded`。専用OpenAI providerと共通内容cacheも
#1060が所有する。以下は専用providerの契約であり、既存OpenAI-compatible VLMのscore閾値・
self-hostの任意APIキー契約は維持する。

### 9.1 データ分類

- Feature名: OpenAI ModerationとCN動画フレーム抽出、内容判定再利用。
- Durable / Transient: 元動画とposterは既存のdocs/blobs。抽出frame・HTTP入力/生応答はtransient。
  完了した内容判定と最小coverage、参照元のverdict/advisoryはnode-local Postgres。
- Canonical Source: 投稿とmediaは既存docs/blobs。内容判定は派生情報であり真の分類を表明しない。
- Replicated?: 内容cacheは複製しない。advisory配布は既存visibility規則に従う。
- Rebuildable From: 認可された元内容と同一provider/抽出構成。frameをcanonical blobにしない。
- Public Replica / Private Replica / Local Only: cacheとdecoder一時領域はCN local only。
- Gossip Hint: 追加なし。Blob: 新たな派生blobなし。SQLite projection: 追加なし。
- 必須contract: boolean判定、カテゴリ別confidence、coverage、部分失敗、同内容の別参照、
  並行miss、構成変更、restart、scope/撤回/appeal保持、取得/decoder/HTTPの上限とcleanup。
- 必須scenario: benign MP4/WebMのcold scan→別投稿/別著者→restart後の再利用で追加動画取得・
  decode・APIが0。mockによる検知と失敗、production相当imageによる実decodeを併用する。

### 9.2 判定とcoverage

- 専用providerは `omni-moderation-latest` の `/v1/moderations` を使い、
  `COMMUNITY_NODE_VLM_API_KEY` を必須credentialとして明示的に読む。
- 発火はカテゴリboolean。scoreは同カテゴリのconfidenceであり、70閾値を二重適用しない。
  confidenceを閾値回避のために改変しない。旧VLMの判定方式と型で区別する。
- 本文と静止画は別入力。動画はCNが本体から採取したJPEGを1画像/requestで検査する。
  全frame成功後だけ同カテゴリbooleanのOR・scoreのMAXを集約し、元動画blobへ記録する。
- `sexual` はnsfw、一般の非性的カテゴリはobjectionable。本文の `sexual/minors` 検知は
  CSE suspectedとして既存critical経路へ渡し、confirmedには昇格しない。
  画像非対応カテゴリの0は未検査。未知CSAM動画/grooming検査済みを表明しない。
- 音声trackは検査しない。正常な音声スキップと無映像/破損/部分成功を混同しない。
- nsfw/objectionableは§8のadvisory付きAllow・trust寄与0を維持する。known-match、
  Arachnid、critical、appeal、operator訂正の契約は維持する。

### 9.3 抽出と上限

- 動画長を整数時間Dへ正規化し `N=min(8,max(1,ceil(D/5秒)))`、
  `t_i=floor(D*(2i+1)/(2N))` の全区間に分布する決定的時刻を採る。
- 初期対応はMP4/H.264、WebM/VP8・VP9。入力32 MiB、600秒、長辺3840/短辺2160pxまで。
  JPEGは長辺512pxまで、1枚256 KiB、全体2 MiB。全量確保前に取得上限を適用する。
- decoderは検証済みlocal bytesのみを専用tmpfs/pipeで扱い、外部参照・network protocolを
  禁止する。入力値をshellへ埋め込まない。子processへcredentialを継承しない。
- 同時動画job/decoder/動画HTTPは各1、待機16件/60秒。fetch30秒、probe5秒、
  decode合計30秒、HTTP30秒、job全体300秒。timeout/cancel/shutdownでkill・wait・cleanupする。
- 検査はサンプリングであり全フレーム網羅を保証しない。抽出設定・decoder identityをversion化する。

### 9.4 共通再利用と予算

- 内容identityは本文hash、media blob hash。node-localなprovider/model/policy/判定方式/
  前処理・抽出構成のfingerprintと組にする。秘密値と投稿固有のauthor/appealをキーや結果へ混入しない。
- 未完了・失敗は完了cacheに保存しない。同一キーの並行missは実行を共有し、restart後も完了結果を再利用する。
- cache hitの前後も参照元のscope・署名・撤回・送信防止を守り、新参照を別に関連付ける。
  一投稿の削除で他の有効参照を消さず、他著者の履歴やappealをコピーしない。
- 本文・静止画・動画・readiness・retryは同じ有界queue/予算を通る。初期予算案は
  400 RPM / 8,000 RPD / 8,000 TPMと実project枠の小さい方。画像TPMを0扱いしない。
- 429/一時障害は追加2回まで、backoffと全体deadlineを共有する。401/403・入力4xxは
  設定/入力障害として扱う。生API応答・media・credentialをログ/DBへ保存しない。

実装状況と検証は [作業記録](../progress/2026-09-17-1060-openai-video-moderation.md) を参照する。
