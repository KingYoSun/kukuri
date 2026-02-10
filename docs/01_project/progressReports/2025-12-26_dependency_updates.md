# Cargo.toml 依存更新レポート

**作成日**: 2025年12月26日  
**作業者**: Codex  
**カテゴリ**: 依存更新/CI

## 概要
cn-cli / kukuri-tauri の依存ライブラリを最新安定版へ更新し、API 変更に合わせたコード調整と CI の Rust バージョン更新を行いました。

## 実施内容

### 1. Cargo.toml の依存更新
- cn-cli / kukuri-tauri の依存を最新安定版へ更新。
- `bincode` は 3.0.0 が `compile_error!` のため 2.0.1 を維持。

### 2. API 変更対応
- bech32 / rand / secp256k1 などの API 変更に合わせて関連実装を調整。

### 3. CI / ドキュメントの更新
- `.github/workflows/test.yml` の `RUST_VERSION` を 1.86 に更新。
- `docs/03_implementation/docker_test_environment.md` の Rust 1.86 記載へ更新。

### 4. 検証
- `./scripts/test-docker.ps1 rust`
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`
- `gh act --workflows .github/workflows/test.yml --job format-check`

## 影響範囲
- Rust 依存更新に伴う P2P/暗号/エンコード周辺の実装調整。
- CI の Rust ツールチェーン更新により native-test-linux の実行環境が 1.86 に揃う。

## 確認結果
- Rust テスト（Docker）: 成功（dead_code など警告は継続）。
- format-check: 成功（`git clone` の non-terminating warning / pnpm の approve-builds warning は継続）。
- native-test-linux: 成功（`git clone` の non-terminating warning、Vitest の `useRouter` 警告は継続）。
