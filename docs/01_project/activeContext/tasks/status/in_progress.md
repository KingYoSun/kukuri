[title] 作業中タスク（in_progress）

最終更新日: 2025年10月31日

## 方針（2025年09月15日 更新）

- 当面は Nostr リレーとは接続しない（外部インデックスサーバー等の導入時に検討）。
- まず P2P（iroh + iroh-gossip + DHT）で一通りの体験が完結することを最優先。
- kukuri 内部のイベントは全て NIPs 準拠（内部フォーマットは Nostr Event スキーマを準拠・整合）。
- テスト戦略: Tauri v2 では E2E が困難なため、層別テスト（ユニット/結合/契約）＋スモーク最小限に切替。

## 現在のタスク

### Clippy 完全解消

- [ ] `cargo clippy --workspace --all-features -- -D warnings` をゼロエラーで完走させる
  - [ ] `domain/entities/event.rs` の `format!` 最適化と `as_bytes` 警告を解消
  - [ ] `DefaultEncryptionService` / `EventHandler` / `LegacyEventManagerHandle` / `NostrClientManager` に `Default` 実装を追加
  - [ ] Clippy 結果と手順を `phase5_ci_path_audit.md` と `refactoring_plan_2025-08-08_v3.md` に反映

### 巨大ファイル分割

- [ ] `application/services/p2p_service.rs`（約797行）を責務別モジュールへ分割しテストを更新
- [ ] `domain/entities/event.rs`（約1310行）をバリデーション/変換ごとに分割しテストを更新
- [ ] 分割結果を `refactoring_plan_2025-08-08_v3.md` と関連 artefact に反映

### 残存 TODO 実装

- [ ] `infrastructure/p2p/event_distributor.rs` のブロードキャスト/配信経路 TODO を実装
- [ ] `infrastructure/p2p/dht_integration.rs` の実装 TODO を完了
- [ ] `domain/p2p/topic_mesh.rs` の購読処理 TODO を実装
- [ ] `application/services/post_service.rs` のトピック別投稿キャッシュ TODO に対応
- [ ] `src/components/layout/Sidebar.tsx` の未読カウント TODO を実装しテストを追加
- [ ] 残存 TODO の棚卸し結果を `phase5_dependency_inventory_template.md` に追記
