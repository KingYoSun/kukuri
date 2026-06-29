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

- `cargo test -p kukuri-cn-safety`（DB 不要、mock feature）: 37 tests pass。
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
