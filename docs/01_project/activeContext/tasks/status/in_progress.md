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
- 進捗: `trending_metrics_job.rs` の未使用メソッドと `DistributorState::strategy` のテスト専用アクセサを整理し、`scripts/test-docker.ps1 lint` で `cargo clippy -D warnings` を再実行してエラーが消えたことを確認。`gh act --job format-check` と `gh act --job native-test-linux`（`NPM_CONFIG_PREFIX=/tmp/npm-global`, `--container-options "--user root"`）も完走し、Linux ネイティブ経路での Rust/TS/Lint すべて green。
