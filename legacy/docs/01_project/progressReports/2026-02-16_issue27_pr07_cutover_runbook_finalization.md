# Issue #27 PR-07 cutover runbook / finalization

最終更新日: 2026年02月16日

## 概要

Issue #27 の PR-07 スコープとして、検索 PG 移行の最終段階に必要な運用 Runbook を確定した。
本PRではコード挙動は変更せず、cutover 時の運用手順・監視項目・ロールバック証跡・Meili 段階撤去条件を文書化して blast radius を最小化した。

## 実施内容

1. cutover Runbook 詳細化
- 変更: `docs/01_project/activeContext/search_pg_migration/PR-07_cutover_runbook.md`
- 追加内容:
  - 5% -> 25% -> 50% -> 100% のカナリア切替手順
  - 品質/性能ゲート閾値（`overlap@10` / `NDCG@10` / P95）
  - 5分以内ロールバック SQL と証跡項目
  - 障害一次切り分け（DB負荷/拡張障害/グラフ遅延）
  - 再インデックス手順（全量/topic 単位）
  - フラグ変更の監査ログ確認手順
  - 既知障害テンプレート
  - Meili 依存の段階撤去条件と手順

2. 監視ダッシュボード項目の運用固定
- 変更: `docs/03_implementation/community_nodes/ops_runbook.md`
- `search/suggest latency`、`index lag`、`zero-result`、`filter drop`、`shadow` の監視観点を、実メトリクス名ベースで追加。

3. ロードマップ最終化
- 変更: `docs/01_project/activeContext/tasks/priority/search_pg_migration_roadmap.md`
- PR-07 の3項目と共通ゲート2項目を完了へ更新。

4. タスク管理更新
- 変更:
  - `docs/01_project/activeContext/tasks/status/in_progress.md`
  - `docs/01_project/activeContext/tasks/completed/2026-02-16.md`
- PR-07 完了メモと完了チェックを追記。

## 互換性/リスク

- 実装コードは無変更のため、既存の API/DB/runtime への動作影響はない。
- 運用判断を Runbook に一本化したことで、cutover 時の手順逸脱リスクを低減。

## 検証

- `git diff --check`（pass）
- docs-only 更新のため、`AGENTS.md` ルールに従いテスト/ビルド/`gh act` は未実施。

## 変更ファイル

- `docs/01_project/activeContext/search_pg_migration/PR-07_cutover_runbook.md`
- `docs/03_implementation/community_nodes/ops_runbook.md`
- `docs/01_project/activeContext/tasks/priority/search_pg_migration_roadmap.md`
- `docs/01_project/activeContext/tasks/status/in_progress.md`
- `docs/01_project/activeContext/tasks/completed/2026-02-16.md`
- `docs/01_project/progressReports/2026-02-16_issue27_pr07_cutover_runbook_finalization.md`
