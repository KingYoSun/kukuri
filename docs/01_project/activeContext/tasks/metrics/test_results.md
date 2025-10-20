# テスト結果

**最終更新**: 2025年10月21日（担当: Codex）

## 更新概要
- 計測日時: 2025年10月20日〜2025年10月21日。
- 実行環境: Windows 11 + PowerShell。Rust テストは Docker 経由が必要だが、今回の再試行は未完。
- 更新頻度: 火曜・金曜の午前に定期実施。クリティカル修正時は臨時実行。
- レビュー体制: 週次スタンドアップで結果を共有し、失敗は `tasks/context/blockers.md` に記録。
- 詳細手順とサマリーは `docs/01_project/activeContext/tasks/metrics/update_flow.md` を参照。

## テストサマリー

### TypeScript（Vitest）
- コマンド: `pnpm test`（ウォッチなし） / `pnpm exec vitest run --reporter=json`
- 結果: 総数 694 件 / 成功 688 件 / 失敗 0 件 / スキップ 6 件
- 成果物: `docs/01_project/activeContext/artefacts/metrics/2025-10-20-vitest-results.json`
- メモ: `--filter` オプションは Vitest v3.2.4 未対応のため、全件実行で集計。

### TypeScript（Lint）
- コマンド: `pnpm lint`
- 結果: 成功（2025年10月21日再実行）
- 対応: Lint 結果を `build_status.md` / `code_quality.md` に反映済み。`collect-metrics` スクリプト経由で定期収集へ組み込み。

### Rust（ユニット / 統合）
- コマンド: `cargo test`
- 結果: 失敗（Windows DLL 依存による `STATUS_ENTRYPOINT_NOT_FOUND (0xc0000139)`）
- 対応: `scripts/run-rust-tests.ps1` を用いた Docker 実行へ移行中。次回計測で結果を記録予定。

### 統合テスト（P2P）
- コマンド: `./scripts/test-docker.ps1 integration`
- ステータス: 今回は未実施。Phase 5 テストモジュール移行完了後に定期実行へ組み込み予定。

## カバレッジ
- TypeScript: 未計測（Vitest JSON から抽出するスクリプトを `collect-metrics` 構想に含める）。
- Rust: 未計測（`cargo tarpaulin` 系ツールは今後検討）。
- 目標: 最低 70% を維持。測定方法が確定した段階で数値を追加する。

## 次回アクション
- Lint 失敗箇所を修正し、成功結果を再収集。
- Docker 経由の Rust テストを実行し、成功/失敗いずれの場合も数値を記録。
- 統合テストの定期実行タイミングを Phase 5 移行タスクと連携させる。
