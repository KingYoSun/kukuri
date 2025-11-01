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

### 巨大ファイル分割（Phase 5 継続）

- [ ] `infrastructure/p2p/event_distributor.rs` を役割単位（戦略・リトライ・メトリクス等）に分割し、公開 API を `mod.rs` で再輸出
- [ ] `application/services/p2p_service.rs` の 700 行超箇所を `core` / `bootstrap` / `metrics` など Phase 5 計画に沿ったモジュールへ切り出し、既存テスト (`tests.rs`) を更新
- [ ] 分割後に `cargo test`（Rust 全体）と `pnpm test`（該当ユニット）を実行し、リグレッションがないことを確認
- [ ] 変更内容を `refactoring_plan_2025-08-08_v3.md` と `phase5_dependency_inventory_template.md` に反映し、成功指標「700行超のファイル0件」の状態を更新

### ユーザー導線ドキュメント整備

- [ ] UI から到達可能な機能一覧を棚卸しし、`docs/01_project/activeContext/artefacts/` 配下にサマリードキュメントを作成
- [ ] Tauri コマンド呼び出し状況（フロントエンド `invoke` 探索結果）と未使用 API の整理結果をドキュメントへ反映
- [ ] `refactoring_plan_2025-08-08_v3.md` のユーザー導線指標チェックボックスを更新し、未達項目のフォロータスクを連携
- [ ] 作成した資料を `phase5_ci_path_audit.md` / `tauri_app_implementation_plan.md` へリンクし、タスク完了後に in_progress.md を更新予定
