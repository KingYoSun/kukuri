# Mainline DHT Runbook / CLI ブートストラップ PoC レポート
最終更新日: 2025年11月12日

## 概要
- Runbook Chapter10 に CLI ブートストラップリストの既定保存先表と Ops/Nightly 連携節（10.4）を追加し、`apply_cli_bootstrap_nodes` の PoC 手順とログ要件を明文化した。
- `tmp/logs/relay_status_cli_bootstrap_20251112-094500.log` を採取し、`phase5_ci_path_audit.md`・`phase5_user_flow_{summary|inventory}.md`・`phase5_dependency_inventory_template.md` と `docs/01_project/roadmap.md`（MVPトラック/KPI/Week2補足）へ Runbook 完了の証跡を反映した。
- `design_doc.md` / `refactoring_plan_2025-08-08_v3.md` / `tasks/completed/2025-11-12.md` を更新し、Mainline DHT Runbook の完了を Phase5 ステータスに反映。今後のフォーカスを EventGateway 実装へ切り替えた。

## 変更詳細
- Chapter10.1 へ OS 別の `cli_bootstrap_nodes.json` 既定パスを追記し、環境変数 `KUKURI_CLI_BOOTSTRAP_PATH` の扱いと Runbook 参照箇所（Chapter2/6）を整理。10.3 には PoC ログ採取フローと `RelayStatus` からの確認ポイントを追記、10.4 では Ops/Nightly のログ保管と `p2p_metrics_export` 監視手順を明文化。
- `docs/01_project/roadmap.md` の P2P & Discovery 行、Week2 補足、KPI（DHT接続成功率）に PoC 完了とログIDを追加し、技術的マイルストーンの「完了」欄へ Runbook を移動。`phase5_ci_path_audit.md`・`phase5_dependency_inventory_template.md`・`phase5_user_flow_summary.md`・`phase5_user_flow_inventory.md` とも参照を同期。
- `design_doc.md` Phase5 行と `refactoring_plan_2025-08-08_v3.md` の P2P/EventGateway 行を Runbook 完了ベースに更新し、残タスクが EventGateway 実装と `trending_metrics_job` カバレッジであることを明示。

## 実施テスト
- `cd kukuri-tauri/src-tauri && cargo test`
- `cd kukuri-cli && cargo test`
- `cd kukuri-tauri && powershell -Command "pnpm install"`
- `cd kukuri-tauri && powershell -Command "pnpm exec vitest run src/tests/unit/components/RelayStatus.test.tsx"`
