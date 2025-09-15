# activeContext 概要（最終確認日: 2025年09月15日）

このディレクトリは、直近の意思決定・タスク運用・計画中/保管中の資料を集約します。参照順は以下のとおりです。

1. tasks/priority/critical.md（最重要タスクの起点）
2. tasks/status/in_progress.md（着手中の明示）
3. tasks/completed/YYYY-MM-DD.md（完了の記録）
4. context/*（ブロッカー・決定事項・環境・負債）
5. metrics/*（ビルド・品質・テスト）

主要ドキュメント
- iroh-native-dht-plan.md: iroh Mainline DHT 移行計画
- tauri_app_experience_design.md: Tauri アプリの体験設計
- tauri_app_implementation_plan.md: 実装計画
- deprecation/
  - gossip_manager_deprecation.md: GossipManager 廃止の経緯
  - distributed-topic-tracker-plan.md: 旧分散トラッカー計画の保管（廃止）

タスク運用ルール（要点）
- 開始時: `tasks/priority/critical.md` から選定 → `tasks/status/in_progress.md` に移動
- 作業中: `in_progress.md` のみを更新（進捗/メモ）。他ファイルは必要時のみ編集
- 完了時: `tasks/completed/YYYY-MM-DD.md` に追記 → `in_progress.md` から削除 → 重要変更は `docs/01_project/progressReports/` に進捗レポート作成
- ブロッカー: `tasks/context/blockers.md` に記録。解決後は削除

補足
- ドキュメントの日付は `YYYY年MM月DD日`（例: 2025年09月15日）表記で統一
- `docs/SUMMARY.md` から本ディレクトリへ辿れるように維持すること
