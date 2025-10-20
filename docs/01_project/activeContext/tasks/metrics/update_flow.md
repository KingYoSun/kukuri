# メトリクス更新フロー

最終更新日: 2025年10月21日

## 現状サマリー
- `build_status.md`・`code_quality.md`・`test_results.md` を 2025年10月21日時点の結果に更新。最新の数値は各ファイルおよび `log_2025-10.md` を参照。
- TypeScript Lint の未使用変数と Rust テストの Windows 依存は既知の課題として明記し、次回計測で再確認する運用に切り替え。
- 計測に使用した成果物（Vitest JSON 等）は `docs/01_project/activeContext/artefacts/metrics/` 配下へ保存し、再集計が可能な状態を確保。

## 課題
- TypeScript Lint の未修正箇所があり、品質指標を正の値で記録できていない。
- Rust テストが Windows で安定しないため、Docker 実行を既定化する仕組みが未整備。
- 自動化スクリプトやカバレッジ取得が未導入で、手動作業と抜け漏れのリスクが残っている。

## 更新フロー案
1. **取得タイミング**
   - 火曜・金曜の午前に定期取得（週2回）。リリース候補作業中は都度更新を許容。
2. **実行コマンド**
   - TypeScript: `pnpm test`（watch なし）と `pnpm exec vitest run --reporter=json`、`pnpm lint` を実行。
   - Rust: `scripts/run-rust-tests.ps1`（Docker 経由で `cargo test` 実行）と `cargo clippy -- -D warnings` を実行。Windows ネイティブで失敗した場合は既知の制約として記録。
   - 結果 JSON（Vitest / Cargo）からテスト件数・成功率を `jq` で抽出し、`test_results.md` に反映。
   - `rg 'TODO' -g '*.ts'` などのスクリプトで TODO / any / `#[allow(dead_code)]` を再計測し、`code_quality.md` に記録（`scripts/metrics/collect-metrics.{ps1,sh}` で自動取得可能）。
3. **記録手順**
   - 各 md の先頭に更新日と更新担当者（イニシャル）を追記。
   - 変更点が多い場合は `docs/01_project/activeContext/artefacts/` に詳細レポートを保存し、該当 md からリンク。
4. **レビュー**
   - 週次スタンドアップで更新内容を共有し、乖離や異常値を確認。
   - 重大な退行（成功率 < 95% など）があった場合は `tasks/context/blockers.md` に記録。

## 今後の対応
- 上記フローを 2025年10月第4週から運用開始し、1週間フィードバックを受けて改訂。
- 自動化スクリプト（PowerShell/Bash）を `scripts/metrics/collect-metrics.{ps1,sh}` として整備済み。未計測指標（未使用API、未使用インポート）やカバレッジ抽出の拡張を検討。
- 運用開始後 1 ヶ月間、更新履歴を `docs/01_project/activeContext/tasks/metrics/log_2025-10.md` に記録して定着を図る。

## 2025年10月21日 フロー確定メモ
- 2025年10月20日の初回収集結果を各メトリクスファイルへ反映済み。
- TypeScript Lint 失敗箇所と Rust テストの Docker 化は引き続きフォローアップ対象。
- `collect-metrics` スクリプトは設計中。完成までの間は `pnpm test` / `pnpm lint` / `scripts/run-rust-tests.ps1` を手動で実行し、`log_2025-10.md` に取得記録を残す。

## 初回収集トライアル（2025年10月20日）
- `pnpm test --filter unit` は Vitest v3.2.4 で未対応。代替として `pnpm test`（watch 付き）と `pnpm exec vitest run --reporter=json` を使用し、`docs/01_project/activeContext/artefacts/metrics/2025-10-20-vitest-results.json` を取得。集計値: 総数694件 / 成功688件 / スキップ6件 / 失敗0件。
- `pnpm lint` が `src/stores/draftStore.test.ts` の未使用変数 `_localStorageMock` 未対応で失敗。Lint 統計を更新する前に修正が必要。
- `cargo test` は Windows 固有の `STATUS_ENTRYPOINT_NOT_FOUND (0xc0000139)` により失敗。Docker 経由実行が必須である点をフローに明記する必要あり。
  - 対応として `scripts/run-rust-tests.ps1` を追加し、`test-docker.ps1` の Rust 実行をラップして自動化。
- `cargo clippy -- -D warnings` は成功。フロー記載の `cargo clippy -D warnings` は引数解釈で失敗するため、コマンド例を修正する。
- TODO/any 集計結果: TypeScript TODO 1件、`any` キーワード 94件。Rust TODO 21件、`#[allow(dead_code)]` 20件。従来値と乖離しているため測定手順を正式化する。
- テスト結果・Lint 成果物を `docs/01_project/activeContext/artefacts/metrics/` 配下に集約する運用が有効と確認。
