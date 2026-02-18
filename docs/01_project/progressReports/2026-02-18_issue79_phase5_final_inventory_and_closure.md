# Issue #79 Phase 5: 全 manifest 最終棚卸しとクローズ判定

作成日: 2026年02月18日
Issue: https://github.com/KingYoSun/kukuri/issues/79

## 実施概要

Issue #79 の Phase 5 として、対象に定義された全 `package.json` / 全 `Cargo.toml` の最終棚卸しを実施し、Phase 1-4 の更新実績と未更新項目の理由を確定した。未更新項目は理由ごとに follow-up Issue へ紐付けし、#79 の Done Criteria 充足可否を判定した。

## 最終 inventory（全 target manifests）

### package.json（2件）

| Manifest | Phase 1-4 反映 | 判定 |
| --- | --- | --- |
| `kukuri-tauri/package.json` | Phase 1（PR #80）で更新 | 更新済み（1件の後続 patch は Phase 5 で follow-up 化） |
| `kukuri-community-node/apps/admin-console/package.json` | Phase 2（PR #81）で更新 | 更新済み |

### Cargo.toml（12件）

| Manifest | Phase 1-4 反映 | 判定 |
| --- | --- | --- |
| `kukuri-tauri/src-tauri/Cargo.toml` | Phase 3（PR #83）で更新 | 更新済み（major/MSRV 由来の carry-over あり） |
| `kukuri-community-node/Cargo.toml` | Phase 4（PR #84）で更新 | 更新済み（MSRV 由来の carry-over あり） |
| `kukuri-community-node/crates/cn-admin-api/Cargo.toml` | 直接変更なし | 直接更新不要（workspace 依存参照） |
| `kukuri-community-node/crates/cn-bootstrap/Cargo.toml` | 直接変更なし | 直接更新不要（workspace 依存参照） |
| `kukuri-community-node/crates/cn-cli/Cargo.toml` | 直接変更なし | 直接更新不要（workspace 依存参照） |
| `kukuri-community-node/crates/cn-core/Cargo.toml` | 直接変更なし | 直接更新不要（workspace 依存参照） |
| `kukuri-community-node/crates/cn-index/Cargo.toml` | 直接変更なし | 直接更新不要（workspace 依存参照） |
| `kukuri-community-node/crates/cn-kip-types/Cargo.toml` | 直接変更なし | 直接更新不要（workspace 依存参照） |
| `kukuri-community-node/crates/cn-moderation/Cargo.toml` | 直接変更なし | 直接更新不要（workspace 依存参照） |
| `kukuri-community-node/crates/cn-relay/Cargo.toml` | 直接変更なし | 直接更新不要（workspace 依存参照。`tokio-tungstenite 0.28.0` は最新） |
| `kukuri-community-node/crates/cn-trust/Cargo.toml` | 直接変更なし | 直接更新不要（workspace 依存参照） |
| `kukuri-community-node/crates/cn-user-api/Cargo.toml` | 直接変更なし | 直接更新不要（workspace 依存参照） |

## 未更新項目（intentional）と理由

1. Rust toolchain / MSRV 制約
   - `iroh 0.95.1 -> 0.96.1` と `iroh-gossip 0.95.0 -> 0.96.0` は `rust-version: 1.89`
   - `time 0.3.45 -> 0.3.47` と `home 0.5.11 -> 0.5.12` は `rust-version: 1.88`
   - 現行 CI / test-runner は Rust `1.86` 固定のため、true latest を適用できない
   - follow-up: https://github.com/KingYoSun/kukuri/issues/85

2. `kukuri-tauri` の major 更新リスク
   - `reqwest 0.12 -> 0.13`
   - `rand 0.9 -> 0.10`
   - `rand_core 0.9 -> 0.10`
   - `bincode 2 -> 3`
   - API/feature 互換差分が大きく、依存全面更新 Issue から分離
   - follow-up: https://github.com/KingYoSun/kukuri/issues/86

3. Node 側の Phase 後発 patch drift
   - `@tanstack/router-vite-plugin 1.160.2 -> 1.161.0`
   - Phase 5 は docs-only finalization を優先し、後続分離
   - follow-up: https://github.com/KingYoSun/kukuri/issues/87

4. 依存更新に付随する既存技術負債
   - `kukuri-tauri/src-tauri` の `clippy::collapsible_if` 73件
   - 依存更新 PR と分離済み
   - follow-up: https://github.com/KingYoSun/kukuri/issues/82

## 実行コマンドと確認結果（Phase 5）

- `rg --files -g 'package.json'` / `rg --files -g 'Cargo.toml'`
  - 対象 manifest を再列挙し、Issue #79 の inventory（package 2件 + Cargo 12件）と一致。
- `gh pr view 80/81/83/84 --json files,mergedAt,...`
  - Phase 1-4 の更新 manifest を確認。
- `pnpm outdated --format json`（`kukuri-tauri`）
  - `@tanstack/router-vite-plugin` のみ差分あり（follow-up #87）。
- `pnpm outdated --long`（`kukuri-community-node/apps/admin-console`）
  - 出力なし（未更新残なし）。
- `cargo update --dry-run`（`kukuri-tauri/src-tauri`, `kukuri-community-node`）
  - `time/home` の更新候補が Rust 1.88+ 依存であることを確認。
- `cargo info iroh@0.96.1` / `iroh-gossip@0.96.0` / `time@0.3.47` / `home@0.5.12`
  - 各 crate の `rust-version` を確認（1.89 / 1.88）。

## Done Criteria 判定（Issue #79）

- 対象全 manifest の更新実施または未更新理由明記: ✅ 充足
- 段階的 PR で CI green 維持: ✅ 充足（PR #80 / #81 / #83 / #84）
- breaking 項目のエスカレーション / follow-up 化: ✅ 充足（#82 / #85 / #86 / #87）

結論: #79 は Done Criteria を満たし、クローズ可能状態。
