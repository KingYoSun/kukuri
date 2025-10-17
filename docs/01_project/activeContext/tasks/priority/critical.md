# 最重要タスク（Critical）

最終更新日: 2025年10月17日

目的: 今後直近で着手すべき最重要タスクの一覧。着手時は本ファイルから `tasks/status/in_progress.md` へ移動して進捗を管理します。

移動済みメモ
- GitHub Actions ワークフロー失敗調査を `tasks/status/in_progress.md` に移動（2025年10月16日着手）
- Iroh DHT/Discovery 残タスクは `tasks/status/in_progress.md` に移動（2025年09月15日）
- v2 アプリ Phase 7 - Mainline DHT 統合タスク群を `tasks/status/in_progress.md` に移動（2025年10月17日）

方針更新（2025年09月15日）
- 当面は Nostr リレーとは接続しない。まず P2P（iroh + iroh-gossip + DHT）で完結した体験の実現を優先。
- kukuri 内部のイベントは全て NIPs 準拠（Nostr Event スキーマ準拠）。

## 2. Phase 7 Exit / Release 準備
- [ ] Mainline DHT 運用 Runbook を作成し、障害対応・監視手順を `docs/03_implementation/p2p_mainline_runbook.md` に整理。
- [ ] 再接続・再索引シナリオの受け入れ基準を定義し、フェーズ完了条件（テストマトリクス・復旧時間目標）をドキュメント化。
- [ ] Phase 7 の成果をまとめたリリースノート／ユーザー告知計画を策定し、配布チャネルと検証手順を決定。

## 3. ブートストラップ/観測基盤の高度化
- [ ] ブートストラップピアの動的更新機構を PoC し、バックエンドと UI の同期方針を設計。
- [ ] DHT メトリクスの長期蓄積パイプラインを検討し、Prometheus/Grafana 等への連携要件を整理。
- [ ] Gossip/P2P の負荷ベンチマーク計画を立案し、ターゲット指標（レイテンシ/スループット）と測定環境を定義。

運用ルール（再掲）
- 新規着手: 本ファイルから対象を選び、`tasks/status/in_progress.md` へ移動
- 完了時: `tasks/completed/YYYY-MM-DD.md` に追記 → `in_progress.md` から削除 → 重要変更は `docs/01_project/progressReports/` にレポート作成

補足
- 既に完了済みの内容は本ファイルから除去済み（詳細は `tasks/completed/2025-09-15.md` を参照）。
