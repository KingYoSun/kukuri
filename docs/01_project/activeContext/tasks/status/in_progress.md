[title] 作業中タスク（in_progress）

最終更新日: 2025年11月01日

## 方針（2025年09月15日 更新）

- 当面は Nostr リレーとは接続しない（外部インデックスサーバー等の導入時に検討）。
- まず P2P（iroh + iroh-gossip + DHT）で一通りの体験が完結することを最優先。
- kukuri 内部のイベントは全て NIPs 準拠（内部フォーマットは Nostr Event スキーマを準拠・整合）。
- テスト戦略: Tauri v2 では E2E が困難なため、層別テスト（ユニット/結合/契約）＋スモーク最小限に切替。

## 現在のタスク

### Clippy 警告ゼロ体制の復帰

- [x] `domain/entities/event/validation/nip19.rs` の `format!` 文字列を埋め込み式へ置換し、Clippy (`uninlined_format_args`) を解消
- [x] `infrastructure/p2p/dht_integration.rs` の `format!` 文字列を修正し、`AppError::DeserializationError` 周辺のログ表現を統一
- [x] `cargo clippy --all-features -- -D warnings` を `kukuri-tauri/src-tauri` で再実行し、警告ゼロを確認（ログ取得・`phase5_ci_path_audit.md` へ反映）
- [x] `kukuri-cli` 側でも `cargo clippy --all-features -- -D warnings` を実行し、警告ゼロ継続を確認
- [x] Clippy 対応後に `refactoring_plan_2025-08-08_v3.md` の成功指標欄を更新し、再発防止タスクを記録

### ユーザー導線ドキュメント整備

- [ ] UI から到達可能な機能一覧を棚卸しし、`docs/01_project/activeContext/artefacts/` 配下にサマリードキュメントを作成
  - 2025年11月01日: 主要画面（Welcome/Home/Topics/Search/Settings/Debugパネル）とサイドバー導線を確認。未リンク状態の要素（Sidebar「トレンド」「フォロー中」、UserSearchResultsの`/profile/$userId`リンク）を記録済み。
  - 2025年11月01日: `docs/01_project/activeContext/artefacts/phase5_user_flow_inventory.md` を起票し、画面別導線と改善候補を整理。
  - 2025年11月02日: グローバル要素セクションにサイドバー未読バッジ/最終活動更新ロジック、PostComposer の下書き運用、offlineSyncService の同期フローを追記し、TopicPage の最終更新表示不具合をギャップに記録。
- [x] Tauri コマンド呼び出し状況（フロントエンド `invoke` 探索結果）と未使用 API の整理結果をドキュメントへ反映
  - 2025年11月01日: `TauriApi`・`SecureStorageApi`・`p2pApi`・`nostr`ユーティリティの `invoke` 使用箇所を洗い出し、未使用コマンド候補（例: `add_relay`, `subscribe_to_user`, `join_topic_by_name` など）を整理中。
  - 2025年11月01日: `offlineApi` 系コマンドと `syncEngine` の直接 `invoke`（`get_post_metadata` など）を棚卸しし、`phase5_user_flow_inventory.md` の 3.1/3.2 に追記。
  - 2025年11月02日: `invokeCommand` / `invoke` 呼び出しをスクリプトで抽出し、統合テスト専用コマンド群（`import_key` ほか）と未使用 API を `phase5_user_flow_inventory.md` 3.2/3.3 に反映、併せて 1.6/3.1 の補足内容を更新。
- [ ] `refactoring_plan_2025-08-08_v3.md` のユーザー導線指標チェックボックスを更新し、未達項目のフォロータスクを連携
  - 2025年11月01日: 「UIから到達可能な全機能の文書化完了」を達成済みに更新し、参照ドキュメントと更新日を記録。
  - 2025年11月02日: 指標欄に最新ドキュメント更新（統合テスト専用コマンド整理）と未導線APIの整理状況を追記、Phase2.5セクションへ `TopicPage` 最終更新バグの改善候補を登録。
- [ ] 作成した資料を `phase5_ci_path_audit.md` / `tauri_app_implementation_plan.md` へリンクし、タスク完了後に in_progress.md を更新予定
  - 2025年11月01日: `phase5_ci_path_audit.md` に関連ドキュメントリンクを追加し、`tauri_app_implementation_plan.md` Phase 5 セクションから参照を追記。
