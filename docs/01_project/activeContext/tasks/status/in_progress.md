[title] 作業中タスク（in_progress）

最終更新日: 2025年11月19日

## 方針（2025年09月15日 更新）

- 当面は Nostr リレーとは接続しない（外部インデックスサーバー等の導入時に検討）。
- まず P2P（iroh + iroh-gossip + DHT）で一通りの体験が完結することを最優先。
- kukuri 内部のイベントは全て NIPs 準拠（内部フォーマットは Nostr Event スキーマを準拠・整合）。
- テスト戦略: Tauri v2 では E2E が困難なため、層別テスト（ユニット/結合/契約）＋スモーク最小限に切替。

## 現在のタスク
### GitHub Actions ワークフロー失敗調査（担当: Codex）
- 状況: `gh act` でローカル再現しつつ、ワークフロー失敗要因を特定・修正する。
- メモ: 2025年11月19日着手。失敗ログの解析と修正内容は作業完了時に追記する。
- 進捗: Rust 側で `P2PEvent` に追加された `topic_id` を未設定だった箇所を修正し、`cargo clippy -D warnings` を遮っていた未使用インポート警告も `state.rs`/`topic_handler.rs` で解消。`gh act --job format-check` は成功、`native-test-linux` は CLI の Docker 依存テストがローカル権限不足で失敗する点を確認済み（本番Runnerでは権限有り）。
