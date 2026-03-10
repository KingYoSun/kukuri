# GitHub Actions 失敗対応（Smoke Tests / Desktop E2E）

日付: 2026年01月13日

## 概要
- Smoke Tests (Docker) のディスク不足で runner が停止する問題を回避するため、ジョブ冒頭のクリーンアップを追加。
- Desktop E2E の seed DB が最新マイグレーションと不整合だったため、最新状態で再生成。

## 対応内容
- `.github/workflows/smoke-tests.yml`: cleanup ステップを追加。
- `kukuri-tauri/testdata/e2e_seed.db`: 最新マイグレーションで再生成。

## 検証
- `gh act --workflows .github/workflows/test.yml --job format-check`
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`

## 補足
- Desktop E2E の失敗は `migration 20250816044844 was previously applied but has been modified` に起因。
