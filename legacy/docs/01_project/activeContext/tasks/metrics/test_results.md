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
- コマンド: `./scripts/test-docker.sh rust`（内部で `docker compose run rust-test` → `cargo test --workspace --all-features -- --nocapture`。P2P スモークは `ENABLE_P2P_INTEGRATION=0` のためスキップ。）
- 結果: 成功（2025年10月26日）。Docker 上で全 144 ユニット + 契約テスト + Offline/P2P スモークが通過。
- メモ: Windows ネイティブ実行は引き続き `STATUS_ENTRYPOINT_NOT_FOUND` の既知事象のため、Rust テストは Docker 経由での実行を必須とする。

### 統合テスト（P2P）
- コマンド: `./scripts/test-docker.ps1 integration`
- ステータス: 今回は未実施。Phase 5 テストモジュール移行完了後に定期実行へ組み込み予定。

## カバレッジ
- TypeScript: 未計測（Vitest JSON から抽出するスクリプトを `collect-metrics` 構想に含める）。
- Rust: `./scripts/test-docker.sh coverage` → `cargo tarpaulin --locked --all-features --skip-clean --out Json --out Lcov`. 2025年10月26日時点の結果は **25.23%（1630/6460 行）**。成果物: `docs/01_project/activeContext/artefacts/metrics/2025-10-26-153751-tarpaulin.{json,lcov}`。
- 目標: Phase 5 で 50%、Phase 6 完了時に 70% へ。GitHub Actions へ組み込む際は tarpaulin の `--fail-under` を段階的に引き上げる。

## 次回アクション
- Lint 失敗箇所を修正し、成功結果を再収集。
- Docker 経由の Rust テストを実行し、成功/失敗いずれの場合も数値を記録。
- 統合テストの定期実行タイミングを Phase 5 移行タスクと連携させる。
