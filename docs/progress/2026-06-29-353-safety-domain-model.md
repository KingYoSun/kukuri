# #353 段階2: safety domain model / provider abstraction

最終更新日: 2026-06-29

## 位置づけ

Issue #353（Public community node 向け CSAM / critical safety moderation 基盤）の段階2の
最初の単位として、CSAM / critical safety の **pure domain 層**を新規 crate `crates/cn-safety`
に実装した。index 本体・readiness CLI・本番 provider 接続は含まない。

設計の真実源:
- `docs/safety/community-node-critical-safety.md`（§7 verdict model, §8 fail-closed invariants, §12 prerequisites）
- `docs/architecture/moderation-event-trust-semantics.md`（advisory ≠ command, visibility 3段階）

## 本 PR で実装した範囲

新規 crate `crates/cn-safety`（`kukuri-cn-safety`）。DB / network / production credentials に
依存しない pure domain crate。

- domain model（`snake_case` serde, 既存 cn-* crate と統一）
  - `SafetyProviderCapability`（capability.rs）
  - `SafetyAction` / `SafetyCategory` / `Severity` / `Basis` / `Visibility` / `ReasonCode` /
    `SafetyLabel` / `SafetyVerdict` / `ModerationAction`（verdict.rs）
  - `ModerationEventBody`（unsigned canonical）/ `SignedModerationEvent` /
    `ModerationEventSigner` / `issue_signed_event`（event.rs）
  - `SafetyRiskSignal` / `RiskSignalTarget` / `AppealStatus`（signal.rs）
- provider abstraction（provider.rs）
  - `#[async_trait] SafetyProvider`（name / capabilities / scan）
  - `ProviderScanRequest` / `ProviderScanResult` / `ScanOutcome` / `ScanError` / `SubjectKind`
- mock（mock.rs, `mock` feature でのみ有効）
  - `MockSafetyProvider`（subject_id ごとに決定論的結果。fail-closed 用に
    failed / unavailable / error も設定可能）
  - `MockSigner`（canonical body の決定論的ハッシュを署名として返す。実鍵ではない）
  - **production の既定 API には含めない**。`MockSigner` は非暗号（FNV-1a）であり本番経路で
    署名として誤用しないよう `#[cfg(feature = "mock")]` で gate する。テストは self
    dev-dependency（`features = ["mock"]`）で有効化する。
- policy router（policy.rs）
  - `route(&[ProviderScanResult], &SafetyPolicy, scanned_at) -> SafetyVerdict` 純関数
  - `SafetyPolicy`（閾値・on_scan_error 等）/ `SafetyPolicy::public_node_default()`
  - fail-closed 保証: unscanned / scan failure / provider unavailable は `Allow` にしない。
    policy が誤って `on_scan_error = Allow`（indexable）でも `ensure_non_indexing` で Hold に倒す。
  - mandatory known CSAM provider の欠落防止: `require_known_csam=true` で
    `KnownCsamHashMatch` capability の scan 結果が無い場合、general moderation が clean/allow でも
    `ProviderUnavailable` として fail-closed する。
  - critical safety の取りこぼし防止: critical capability / critical label を持つ `Completed`
    検知は、suspected 閾値未満や `score=None` でも `Allow`/`Clean` に落とさず fail-closed する
    （`is_critical_detection` + 最終 critical 取りこぼしガード）。
  - suspected 判定の実効スコアは `result.score` または critical label の最大 `confidence`
    （`effective_critical_score`）。score と label confidence の独立性による取りこぼしを防ぐ。
  - reason / category は critical label を優先し、無ければ capability から導く
    （`critical_category`）。CSE を CSAM と取り違えない。
  - `NoKnownMatch` を `Clean` と同一視しない。

配線:
- ルート `Cargo.toml` の workspace members に `crates/cn-safety` を追加。
- `xtask` の `CN_PACKAGES` に `kukuri-cn-safety` を追加（`cargo xtask cn-check` / `cn-test` 対象）。

テスト（DB 不要・決定論的、`mock` feature 有効）:
- `tests/domain_model.rs`: serde round-trip、`csam_confirmed` ≠ `csam_suspected`、
  `NoKnownMatch` ≠ `Clean`、canonical 決定性、mock signer 決定性と body/署名分離、
  `is_indexable()` が `Allow` のときだけ true。
- `tests/policy_router.rs`: known match → exclude/critical/confirmed、suspected →
  quarantine/critical（confirmed と別）、general route 分離（critical=false）、
  scan failure / unavailable / unscanned の fail-closed、mock provider の決定論的挙動。
  追加: 閾値未満の critical 検知 / `score=None` の critical 検知が fail-closed すること、
  label confidence のみで suspected になること、CSE が CsamSuspected に誤分類されないこと。

## 受け入れ条件（Issue #353）との対応

本 PR で満たす:
- [x] moderation server / provider abstraction の設計と実装（abstraction + mock。server 本体は後続）
- [x] known CSAM hash match capability を表現できる
- [x] unknown CSAM / CSE suspected を confirmed と分けて扱える
- [x] general moderation と critical safety route が分離されている
- [x] scan failure / provider unavailable 時に fail-closed する（router レベル）
- [x] signed moderation event を生成できる（型 + signer 抽象 + mock。実鍵 / 永続化は後続）
- [x] trustness / relation に反映する risk signal 型を定義
- [x] mock provider によるテストがある

後続 PR に残す:
- [ ] community node が scan 前 media を index しない（indexing 本体 + DB 制約）
- [ ] `allow` 以外が search / discovery / recommendation に入らない（indexing 本体）
- [ ] public-node readiness check / `safety` CLI（known CSAM provider 必須検査など）
- [ ] provider credential 未設定の public-node profile が起動 / readiness で失敗する
- [ ] blob 本体を恒久保存しないことのテスト / 設定保証
- [ ] #391 本番 provider 接続（Project Arachnid Shield）と moderation event の実鍵署名（secp256k1）

## prerequisites（critical-safety doc §12）との対応

- [x] provider abstraction and provider capability modeling
- [x] mock provider coverage for deterministic tests
- [x] policy routing that separates known CSAM / suspected unknown CSAM / general moderation
- [ ] readiness checks for public-node profiles（後続）
- [ ] fail-closed indexing constraints（router の verdict は実装済み。DB 制約は後続）
- [x] signed moderation event generation（型 + mock signer。実鍵は後続）
- [x] risk signal persistence and distribution semantics の型（永続化は後続）

## 検証

- `cargo test -p kukuri-cn-safety`（DB 不要、mock feature）: 38 tests pass。
- `cargo check -p kukuri-cn-safety --no-default-features`（production ビルド = mock 無し）: pass。
- `cargo clippy -p kukuri-cn-safety --all-targets --all-features -- -D warnings`: clean。
- `cargo fmt -p kukuri-cn-safety --check`: clean。
- `cargo xtask cn-check`（CN slice の check）。

## 既知の制約 / 注意

- `canonical_bytes()` は `serde_json` の決定論的出力（struct 宣言順、map 不使用）に依存する。
  同一バージョン内での決定性を担保する。クロス実装で厳密一致が必要なら、実鍵署名導入時に
  canonical 形式（フィールドソート等）を強化する。
- `MockSigner` の署名は FNV-1a ベースで暗号学的強度を持たない。テスト専用であり `mock`
  feature でのみ公開する（production の既定 API には出さない）。
- `Availability::Planned`（`Moderation` / `CommunityIndex` / `CommunityLocalTrust`）は本 PR では
  変更しない。public indexing の解禁は段階3完了まで行わない（docs の制約を維持）。

## マージ後の整理（次段階への引き継ぎ）

PR #396（`9b13c717 feat(cn): add safety domain model`）は `main` にマージ済み。
この段階で追加された `crates/cn-safety` は意図的に **pure domain / pure policy crate** に留めており、
community node runtime へはまだ接続していない。

### 現在の到達点

- `crates/cn-safety` は workspace / `xtask` の CN package set に入っている。
- `cn-safety` は provider 抽象・mock provider・verdict domain model・signed event 型・risk signal 型・
  policy router を提供する。
- router レベルでは fail-closed が実装済み:
  - unscanned / scan failure / provider unavailable は `Allow` にならない。
  - `require_known_csam=true` で `KnownCsamHashMatch` capability の scan 結果が無い場合は fail-closed。
  - critical capability / critical label の `Completed` 結果は、score 欠落や閾値未満でも `Clean` / `Allow` に落とさない。
  - `NoKnownMatch` は `Clean` と同一視しない。
- mock は `mock` feature でのみ公開し、非暗号の `MockSigner` を production の既定 API から外している。

### 次段階で重要な現状確認

- `cn-core` / `cn-operator` / `cn-cli` / `cn-user-api` はまだ `kukuri-cn-safety` に依存していない。
  つまり次段階の最初の作業は、runtime / operator / CLI のどこからこの domain crate を使うかの接続設計になる。
- `cn-core` には現時点で community index / search / discovery / recommendation の実体が無い。
  そのため「fail-closed indexing」は既存 index への制約追加ではなく、index 機構や persistence boundary の新設を含む可能性が高い。
- `cn-operator` の `CommunityIndex` / `Moderation` / `CommunityLocalTrust` は引き続き `Availability::Planned`。
  runtime 側の readiness / fail-closed indexing / signed event 永続化が揃うまで、これらを available として扱わない。
- #391（Project Arachnid Shield 統合）は未完了。今回の crate は #391 の本番 provider 実装が差し込む trait surface を用意した段階。

### 推奨する次 PR の切り方

1. **safety runtime adapter / scan orchestration**
   - `cn-safety` の `SafetyProvider` / `SafetyPolicy` / `route()` を呼び出す境界を作る。
   - provider 呼び出し失敗を `ScanOutcome::Failed` / `Unavailable` に写像し、router の fail-closed を runtime で使えるようにする。
   - まだ本番 provider は接続せず、mock / test provider で contract を固定する。

2. **fail-closed indexing contract**
   - index 対象レコードに safety verdict / scan state を必ず伴わせる contract を先に置く。
   - `Allow` 以外、unscanned、scan_failed、provider_unavailable が search / discovery / recommendation に出ないことをテストで固定する。
   - `cn-core` に index 実体が無いため、schema / storage / query boundary の設計から始める。

3. **public-node readiness / safety CLI**
   - known CSAM provider 設定、credential 有無、`index_before_scan=false`、`on_scan_error=hold`、signed event 有効化、blob 恒久保存なしを検査する。
   - `cn-operator` の planned capability を available にする前の gate として使う。

4. **signed moderation event persistence / real signer**
   - `ModerationEventBody` + `ModerationEventSigner` の抽象を使い、issuer node の実鍵署名と保存 boundary を追加する。
   - 現在の `canonical_bytes()` は同一実装内の決定性に留まるため、実鍵署名導入時に canonicalization を強化する。

5. **#391 provider integration**
   - Project Arachnid Shield などの本番 provider を `SafetyProvider` trait 実装として追加する。
   - credential / timeout / provider unavailable を readiness と router fail-closed に接続する。

### 次段階で避けること

- runtime / indexing が fail-closed になる前に `Moderation` / `CommunityIndex` を available 扱いにしない。
- `NoKnownMatch` を `Clean` の証明として扱わない。
- `MockSigner` を本番署名として使わない。
- general NSFW / spam moderation と CSAM / CSE critical route を同じ queue / reason code にまとめない。

## 段階3a: readiness + safety CLI + config schema

indexing 本体（DB / storage / search / discovery / recommendation 除外制約）に入る前の DB 非依存作業として、`cn-operator` から `cn-safety` を初めて参照する結線を追加した。

### 実装範囲

- `operator-config.yaml` の schema に `safety` セクションを追加。
  - `safety.profile` / `policy_version`
  - `safety.indexing.index_before_scan` / `on_scan_error`
  - `safety.storage.permanent_blob_storage`
  - `safety.events.emit_signed_moderation_events`
  - `safety.providers.known_csam` / `general` / `unknown_csam`
  - provider credential は値ではなく `credential_secret_id`（Secret Manager secret ID 参照）のみ。
  - `SafetyProviderEntry.on_high_confidence` は宣言として受理・検証するのみ。**現段階の readiness
    判定には未使用**で、実際の効果は後続の runtime scan orchestration で適用する。
- `cn-operator` に `kukuri-cn-safety` 依存を追加。
  - default build は `cn-safety/mock` を有効化しない。
  - `safety-mock` feature でのみ `MockSafetyProvider` 経路を使う。
- public-node readiness 判定を `cn-operator` の pure logic として追加。
  - `Pass` / `Fail` / `Unknown` の3状態。
  - readiness check の id は `READINESS_CHECK_IDS`（単一の真実源）に集約し、safety セクション有無の
    両経路が同じ id 集合を同順で網羅する。
  - `ReadinessReport::is_ready()` は全 check が `Pass` の場合のみ true（最終的な readiness 完了）。
  - `ReadinessReport::static_checks_pass()` / `has_blocking_failures()` は static config の不備有無を表す
    （`Unknown` は blocking 失敗として扱わない）。
  - `Unknown`（credential 実検証、scan coverage metrics）は pass 扱いしない。
- `cn-operator safety readiness` を追加。
  - operator-config を読み、readiness check を一覧表示する。
  - exit code: static check に `Fail` がある場合のみ failure。`Fail` が無く `Unknown` だけが残る場合は
    success とし、「runtime / provider 接続後に再検査が必要」である旨を NOTE として出力する。
    これにより、正しく設定された public-node が（runtime 未接続を理由に）常に failure になることを防ぐ。
  - 出力に `ready`（最終完了）と `static_ok`（設定上の不備なし）の両方を表示する。
- `cn-operator safety test-provider` を追加。
  - `safety-mock` feature 有効時のみ mock provider で scan → route → verdict を実行。
  - feature 無効時は再ビルド案内を出して failure exit code。

### readiness の現在の判定項目

config から静的に判定する項目:

- `safety` セクションが存在すること。
- `safety.profile` が `public-node`（未指定なら public-node とみなす）。
- known CSAM provider が設定されていること。
- known CSAM provider が `required: true` であること。
- `index_before_scan=false`。
- `on_scan_error` が `allow` ではないこと。
- signed moderation event が有効であること。
- permanent blob storage が無効であること。
- known CSAM provider の `credential_secret_id` が設定され、secret ID 形式として妥当であること。

runtime / provider 接続後に残る `Unknown`:

- provider credential の実検証。
- scan coverage metrics の利用可能性。

### 明示的に維持している境界

- `Moderation` / `CommunityIndex` / `CommunityLocalTrust` は引き続き `Availability::Planned`。
- readiness は public indexing 解禁前の gate であり、indexing 本体や runtime 接続ではない。
- signed moderation event の永続化と実鍵署名は未実装。
- #391 Project Arachnid Shield の本番 provider 接続は未実装。

### 検証

- `cargo test -p kukuri-cn-operator`: pass（default build。`safety` schema / readiness static checks）
- `cargo test -p kukuri-cn-operator --features safety-mock`: pass（mock provider 経路を含む）
- `cargo clippy -p kukuri-cn-operator --all-targets -- -D warnings`: clean（default build）
- `cargo clippy -p kukuri-cn-operator --all-targets --all-features -- -D warnings`: clean
- `cargo fmt -p kukuri-cn-operator --check`: clean
- `cargo xtask cn-check`: pass
- `cargo run -p kukuri-cn-operator --features safety-mock -- safety test-provider --scenario known-match --subject-id blob-1`: known match → `Exclude` / `CsamConfirmed` / `indexable=false`

### レビュー指摘への対応（2026-06-29 追記）

- readiness の `is_ready()` が runtime 未確定の `Unknown` により常に false になり、`safety readiness` CLI が
  完全設定でも常に failure exit code を返していた問題を是正。`static_checks_pass()` /
  `has_blocking_failures()` を追加し、CLI は `Fail` がある場合のみ failure、`Unknown` のみ残る場合は
  success + NOTE を返すようにした。出力に `ready` と `static_ok` を併記。
- readiness check id を `READINESS_CHECK_IDS` 定数に集約し、通常経路と safety 欠落経路の id 集合
  不一致（`safety_profile_public_node` の欠落）を解消。両経路が同一 id を同順で網羅することをテストで固定。
- 非 public-node profile / 空 profile の readiness 挙動にテストを追加（早期 return 分岐のカバレッジ）。
- `SAMPLE_CONFIG` の `general` / `unknown_csam` provider を `placeholder-*` に変更し、本番値でないことを
  コメントで明示。
- `SafetyProviderEntry.on_high_confidence` が現段階では readiness 未使用である旨をコード doc コメントと
  本 doc に明記。

### レビューでの修正

- `safety_readiness.rs` の `impl From<SafetyErrorAction> for kukuri_cn_safety::SafetyAction` を削除した。
  消費箇所が無く（`on_scan_error` → `SafetyAction` の写像は後続の runtime scan adapter で使う想定だが、本 PR スコープ外）、
  `pub` trait impl は `dead_code` lint の対象外のため検出されず残っていた。dead code を残さない方針に従い除去した。

## 段階3b: safety runtime adapter / scan orchestration

indexing 本体に入る前の DB 非依存作業として、`cn-safety` の pure domain（`SafetyProvider` /
`SafetyPolicy` / `route()`）を実際に駆動する境界を新規 crate `crates/cn-safety-runtime`
（`kukuri-cn-safety-runtime`）として追加した。

### 実装範囲

- 新規 crate `crates/cn-safety-runtime`（`cn-safety` にのみ依存。DB / network / production
  credentials なし）。
  - workspace members と `xtask` `CN_PACKAGES` に追加（`cargo xtask cn-check` / `cn-test` 対象）。
- 抽象注入:
  - `ScanClock { fn now_rfc3339(&self) -> String }` … scan 時刻供給。orchestrator は clock 注入のまま維持。
    本番実装 `SystemScanClock`（system clock, UTC RFC3339）を追加済み（#398）。
  - `EventIdGenerator { fn next_id(&self) -> String }` … moderation event id 供給。
    本番実装 `UuidEventIdGenerator`（UUID v4）を追加済み（#399）。
  - テストは orchestrator 経路を固定 clock / 連番 id で決定論的に検証し、`SystemScanClock` は
    RFC3339 / UTC / 秒精度の契約を別 contract test で検証。
- `SafetyOrchestrator`（builder で provider を登録順に保持）:
  - `scan_subject(&ProviderScanRequest) -> SafetyScanReport`。
  - provider を**登録順に逐次実行**し、各 `ProviderScanResult` を集約して 1 回 `route()` に渡す。
  - provider が `Err` を返した場合、`map_scan_error` で `ScanOutcome` に写像し、provider 名 /
    capability を保持した `ProviderScanResult` を**合成**する（結果集合から除外しない）。
    一部成功 + 一部失敗の取りこぼしを防ぐ fail-closed の要。
  - build 時に issuer node id 空 / provider 不在 / capability 無し provider / 空 provider 名を
    `SafetyRuntimeError` で拒否。
- `ScanError → ScanOutcome` 写像:
  - `Unavailable` → `Unavailable`、`Timeout` / `Protocol` → `Failed`。route() の既存 fail-closed
    分岐（Unavailable→ProviderUnavailable, Failed→ScanFailed）と整合。
- verdict からの**未署名** moderation artifact 生成（`SafetyScanReport.moderation_event` /
  `risk_signal`）:
  - indexable（`allow`）verdict では artifact を生成しない。
  - target（subject_kind / subject_id）が欠けている場合や `subject_id` が空/空白の場合は artifact を生成しない（空 target_id の
    moderation event を作らない）。
  - operational fail-closed（scan_failed / provider_unavailable / unscanned）は content の safety
    category を示さないため **risk signal を作らない**（虚偽の risk label を作らない）。moderation
    event は target が揃えば生成（visibility=local）。
  - visibility は `SafetyRiskSignal::default_visibility_for(category, basis)` に従い、suspected
    unknown CSAM / CSE は `Local` 既定、confirmed のみ `SubscribedNodes` 以上。
  - critical reason（`csam_confirmed` / `csam_suspected` / `cse_suspected`）は reason_code を優先して category を導出し、provider が先頭に一般 label を返しても critical category を取り違えない。
  - 署名はしない（`SignedModerationEvent` は作らない。body のみ）。

### 維持した境界

- 本番 provider 接続（#391）、実鍵署名（secp256k1）、event / signal の永続化は未実装。
- fail-closed indexing 本体（DB 制約）、moderation server の HTTP / blob 一時 fetch は未実装。
- `cn-operator` / `cn-core` からの本 crate 利用結線（runtime 組み込み）は後続。
- `Moderation` / `CommunityIndex` / `CommunityLocalTrust` は引き続き `Availability::Planned`。
- mock（provider / clock / id）は production の既定 API に出さない（cn-safety の mock は
  dev-dependency の `features = ["mock"]` 経由、runtime の clock / id mock は integration test 内
  local 定義）。

### 後続への申し送り（別 Issue）

- 本番 `ScanClock`（system clock, RFC3339）実装 … Issue #398（`SystemScanClock` として実装済み）。
- 本番 `EventIdGenerator`（UUID / ULID）実装 … Issue #399（`UuidEventIdGenerator` として実装済み）。
- runtime 組み込み時は `SystemScanClock` と `UuidEventIdGenerator` を注入する。

### 検証

- `cargo test -p kukuri-cn-safety-runtime`（mock provider / 固定 clock / 連番 id、DB 不要 + SystemScanClock / UuidEventIdGenerator contract）: 23 tests pass。
- `cargo check -p kukuri-cn-safety-runtime --no-default-features`（production ビルド = mock 無し）: pass。
- `cargo clippy -p kukuri-cn-safety-runtime --all-targets --all-features -- -D warnings`: clean。
- `cargo fmt -p kukuri-cn-safety-runtime --check`: clean。
- `cargo xtask cn-check`（CN slice。新 crate を含む）。
- `cargo xtask cn-test`（CN slice の test。Postgres/Valkey harness 起動を含む）。
- `cargo test -p xtask`（`CN_PACKAGES` 長変更の回帰確認）。

## #398: 本番 ScanClock（SystemScanClock）

cn-safety-runtime に system clock ベースの本番 `ScanClock` 実装 `SystemScanClock` を追加した。

### 実装範囲

- `crates/cn-safety-runtime/src/clock.rs` に `SystemScanClock` を追加。
  - `now_rfc3339()` は `chrono::Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)` を返す。
  - UTC・秒精度・`Z` suffix に正規化（例: `2026-06-29T09:00:00Z`）。監査時刻として秒未満は不要。
- crate root から `SystemScanClock` を re-export。`Arc<dyn ScanClock>` として
  `SafetyOrchestrator::builder` に渡せる。
- `chrono` を `cn-safety-runtime` の通常依存に追加（workspace dependency。新規外部 dependency は増やさない）。
- `ScanClock` trait と orchestrator のシグネチャは変更しない。テストの固定 clock も従来どおり。

### 検証

- `cargo test -p kukuri-cn-safety-runtime --test clock`: 3 tests pass（RFC3339 / UTC / 秒精度 / orchestrator 注入）。
- `cargo check -p kukuri-cn-safety-runtime --no-default-features`: pass。
- #399（本番 `EventIdGenerator`）も対応済み。下記 #399 節を参照。

## #399: 本番 EventIdGenerator（UuidEventIdGenerator）

cn-safety-runtime に UUID v4 ベースの本番 `EventIdGenerator` 実装 `UuidEventIdGenerator` を追加した。

### 実装範囲

- `crates/cn-safety-runtime/src/id.rs` に `UuidEventIdGenerator` を追加。
  - `next_id()` は `uuid::Uuid::new_v4().to_string()`（ハイフン付き小文字 UUID v4）を返す。
- crate root から `UuidEventIdGenerator` を re-export。`Arc<dyn EventIdGenerator>` として
  `SafetyOrchestrator::builder` に渡せる。
- `uuid` を `cn-safety-runtime` の通常依存に追加（workspace dependency。lockfile 解決済みで
  新規外部 dependency は増やさない）。
- ULID ではなく UUID を採用。`uuid` が `features=["serde","v4"]` 付きで既存 workspace dep のため、
  追加コストなく v4 を利用できる（`ulid` は未導入で「新規外部 dependency を増やさない」方針に反する）。
- `EventIdGenerator` trait と orchestrator のシグネチャは変更しない。テストの連番 id 生成器も従来どおり。

### 検証

- `cargo test -p kukuri-cn-safety-runtime --test id`: 3 tests pass（UUID v4 parse / 一意性 / orchestrator 注入）。
- `cargo check -p kukuri-cn-safety-runtime --no-default-features`: pass。