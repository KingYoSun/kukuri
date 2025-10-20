# メトリクス更新フロー案

最終更新日: 2025年10月20日

## 現状サマリー
- 全ファイルとも最終更新日が 2025年08月16日で止まっており、最新の Phase 7 作業状況と乖離。
- `build_status.md`: TypeScript/Rust/Tauri の結果を定性的に記録。Rust は警告件数が旧構成のまま。
- `code_quality.md`: TODO や dead_code のカウントを静的記録しているが、算出方法のメモが無く再現が困難。
- `test_results.md`: Windows 既知課題や統合テスト未実装の備考は現状と整合しているが、テスト数・成功率は旧結果。

## 課題
- 計測タイミングと手順が文書化されていないため、担当者によって取得方法がばらつく。
- 取得元（`pnpm test`, `cargo test`, `cargo clippy`, CI ログなど）が明示されておらず、再現が難しい。
- 更新履歴がなく、どの変更がいつ反映されたか追跡しづらい。

## 更新フロー案
1. **取得タイミング**
   - 火曜・金曜の午前に定期取得（週2回）。リリース候補作業中は都度更新を許容。
2. **実行コマンド**
   - `pnpm test --filter unit` と `pnpm lint` の結果を `build_status.md` へ反映。
   - `cargo test`（Windows/WSL いずれか）と `cargo clippy -D warnings` の結果を `build_status.md` と `test_results.md` に反映。
   - `pnpm test -- --json` と `cargo test --message-format=json` を `jq` で要約し、テスト件数・成功率を計算。
   - `rg 'TODO' -g '*.ts'` などのスクリプトで TODO/any/dead_code を再計測し、`code_quality.md` に記録。
3. **記録手順**
   - 各 md の先頭に更新日と更新担当者（イニシャル）を追記。
   - 変更点が多い場合は `docs/01_project/activeContext/artefacts/` に詳細レポートを保存し、該当 md からリンク。
4. **レビュー**
   - 週次スタンドアップで更新内容を共有し、乖離や異常値を確認。
   - 重大な退行（成功率 < 95% など）があった場合は `tasks/context/blockers.md` に記録。

## 今後の対応
- 上記フローを 2025年10月第4週から運用開始し、1週間フィードバックを受けて改訂。
- 自動化スクリプト（PowerShell/Bash）を `scripts/metrics/collect-metrics.{ps1,sh}` として整備し、手動作業を短縮。
- 運用開始後 1 ヶ月間、更新履歴を `docs/01_project/activeContext/tasks/metrics/log_2025-10.md` に記録して定着を図る。

## 初回収集トライアル（2025年10月20日）
- `pnpm test --filter unit` は Vitest v3.2.4 で未対応。代替として `pnpm test`（watch 付き）と `pnpm exec vitest run --reporter=json` を使用し、`docs/01_project/activeContext/artefacts/metrics/2025-10-20-vitest-results.json` を取得。集計値: 総数694件 / 成功688件 / スキップ6件 / 失敗0件。
- `pnpm lint` が `src/stores/draftStore.test.ts` の未使用変数 `_localStorageMock` 未対応で失敗。Lint 統計を更新する前に修正が必要。
- `cargo test` は Windows 固有の `STATUS_ENTRYPOINT_NOT_FOUND (0xc0000139)` により失敗。Docker 経由実行が必須である点をフローに明記する必要あり。
  - 対応として `scripts/run-rust-tests.ps1` を追加し、`test-docker.ps1` の Rust 実行をラップして自動化。
- `cargo clippy -- -D warnings` は成功。フロー記載の `cargo clippy -D warnings` は引数解釈で失敗するため、コマンド例を修正する。
- TODO/any 集計結果: TypeScript TODO 1件、`any` キーワード 94件。Rust TODO 21件、`#[allow(dead_code)]` 20件。従来値と乖離しているため測定手順を正式化する。
- テスト結果・Lint 成果物を `docs/01_project/activeContext/artefacts/metrics/` 配下に集約する運用が有効と確認。
