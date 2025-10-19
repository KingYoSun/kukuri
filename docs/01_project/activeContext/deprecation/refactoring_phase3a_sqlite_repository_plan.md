# Phase 3A 実行計画 - SqliteRepository 分割
最終更新日: 2025年10月19日

## 1. 目的とスコープ
- Phase 3A では `kukuri-tauri/src-tauri/src/infrastructure/database/sqlite_repository.rs`（1099行）を分割し、責務ごとのモジュール構成へ移行する。
- 目標は以下の3点:
  1. Repository トレイト実装の可読性向上（Post/Topic/User/Event の責務を明確化）。
  2. SQL → ドメイン変換ロジックの重複解消と将来の DRY 化（Phase 4 で追加予定）に備える。
  3. `.sqlx/` オフラインデータの更新手順を確立し、DB スキーマ変更時の影響範囲を把握しやすくする。
- 本フェーズでは Rust 側の API 互換性を維持し、呼び出し元（サービス層・ハンドラー・DI コンテナ）の変更を必要最小限に抑える。

## 2. 現状分析

### 2.1 ファイル構造と規模
| Impl | 行数 (概算) | 提供関数 |
| --- | --- | --- |
| `impl Repository for SqliteRepository` | 15 | `initialize`, `health_check` |
| `impl PostRepository for SqliteRepository` | 324 | `create_post`, `get_post`, `get_posts_by_topic`, `update_post`, `delete_post`, `get_unsync_posts`, `mark_post_synced`, `get_posts_by_author`, `get_recent_posts` |
| `impl TopicRepository for SqliteRepository` | 299 | `create_topic`, `get_topic`, `get_all_topics`, `get_joined_topics`, `update_topic`, `delete_topic`, `join_topic`, `leave_topic`, `update_topic_stats` |
| `impl UserRepository for SqliteRepository` | 184 | `create_user`, `get_user`, `get_user_by_pubkey`, `update_user`, `delete_user`, `get_followers`, `get_following` |
| `impl EventRepository for SqliteRepository` | 258 | `create_event`, `get_event`, `get_events_by_kind`, `get_events_by_author`, `delete_event`, `get_unsync_events`, `mark_event_synced`, `add_event_topic`, `get_event_topics` |

補足メトリクス:
- `sqlx::query(` 呼び出し 40 箇所、`chrono::DateTime::from_timestamp_millis` 15 箇所、`Row::try_get` 71 箇所。Row → ドメイン変換が分散している。
- `serde_json::from_str` 等を Topic/Event 双方で複数回実装しており、タグマッピングやカウント処理が重複。

### 2.2 依存と利用箇所
- DI: `src/state.rs:135` で `Arc::new(SqliteRepository::new(pool.clone()))` として初期化し、`UserService`/`TopicService`/`PostService`/`EventService` 等へトレイト別に注入。
- Trait 依存: `application/services` 直下の各サービスが `PostRepository` などのトレイトを利用。呼び出しはトレイト経由のため、公開 API を維持すれば呼び出し側変更は不要。
- テスト: 直接 `SqliteRepository` を利用するユニットテストは現状存在せず、移行後は最低限の smoke テスト追加を検討（将来の Phase 4 で対応）。

### 2.3 既知の懸念
- `.sqlx/` ディレクトリが最新 schema を前提に動作しており、分割後も `cargo sqlx prepare` の再実行と差分レビューが必要。
- EventRepository で `add_event_topic` を独自に実装しており、TopicRepository の join/leave とカスケード関係の整理が必要。
- PostRepository の `create_post` では Nostr タグ埋め込みを行っており、ファクトリ化の際に serde 処理を共通化する必要がある。

## 3. 目標モジュール構成

```
src-tauri/src/infrastructure/database/sqlite_repository/
├── mod.rs                 // 公開エントリポイント（構造体定義・コンストラクタ）
├── posts.rs               // PostRepository 実装
├── topics.rs              // TopicRepository 実装
├── users.rs               // UserRepository 実装
├── events.rs              // EventRepository 実装
├── mapper.rs              // Row → ドメイン変換ユーティリティ
└── queries.rs             // SQL 文定義（const &str または include_str!）
```

設計方針:
- `SqliteRepository` 構造体および共通フィールドは `mod.rs` に集約し、サブモジュールでは `super::SqliteRepository` を利用する。
- `mapper.rs` では `map_post_row`, `map_topic_row`, `map_user_row`, `map_event_row` 等の関数を提供し、`Row` からの変換ロジックを一本化。
- `queries.rs` では複雑な SQL を名前付き定数として定義し、各サブモジュールで再利用。将来的な SQL 更新の diff が明確になる。
- サブモジュール間で共通 util が必要な場合は `crate::shared` ではなく同階層のユーティリティに閉じ込める。

## 4. 移行手順

1. **骨組み構築**
   - `sqlite_repository` ディレクトリを作成し、`mod.rs` に `SqliteRepository` 構造体・`new` コンストラクタ・`ConnectionPool` 保持を移動。
   - 既存 `sqlite_repository.rs` から他モジュールがインポートするパスを `sqlite_repository::SqliteRepository` で引き続き解決できるよう `mod.rs` から re-export。
2. **PostRepository の移動**
   - `posts.rs` に PostRepository 実装を丸ごと移し、SQL 文を `queries.rs` に切り出す。
   - `mapper.rs` に Post エンティティ変換ユーティリティを追加し、`get_post`・`get_posts_by_topic` 等で利用。
3. **TopicRepository / UserRepository / EventRepository の段階的移動**
   - 各トレイトごとにブランチを小分けに移行（`topics.rs` → `users.rs` → `events.rs` の順）。移行毎に `cargo fmt` とユニットテストを実行。
   - EventRepository ではタグ情報との連携があるため、`mapper.rs` にタグ抽出ヘルパー（例: `extract_topics_from_tags`）を追加。
4. **共通処理の整理**
   - `serde_json::to_string` 等の重複処理を `mapper.rs` または `queries.rs` に統合。
   - `chrono::DateTime::from_timestamp_millis` の繰り返し箇所を `mapper.rs` 経由で行い、デフォルト値（`Utc::now`）の扱いを一元化。
5. **`.sqlx/` の更新と検証**
   - 変更後に `DATABASE_URL="sqlite:data/kukuri.db" sqlx database create`（必要に応じて既存 DB をバックアップ）→ `sqlx migrate run` → `cargo sqlx prepare` を実行。
   - `.sqlx/` 配下に差分が出た場合はレビュー用に記録し、不要な再生成がないか確認。
6. **終盤確認**
   - `cargo fmt && cargo clippy -D warnings && cargo test` を `kukuri-tauri/src-tauri` で実行。
   - 依存サービス（Auth/Post/Topic/User/Event/Offline）に対して smoke テストまたは手動起動による動作確認を行う。

## 5. 検証計画
- Rust: `cd kukuri-tauri/src-tauri && cargo fmt && cargo clippy -D warnings && cargo test`
- SQLx: `DATABASE_URL="sqlite:data/kukuri.db" sqlx database create` → `sqlx migrate run` → `cargo sqlx prepare`
- TypeScript: 影響は限定的だが回帰確認として `cd kukuri-tauri && pnpm test` を推奨
- 手動: Windows 環境で `./scripts/test-docker.ps1 rust` により Docker 経由テストも確認（既知の `STATUS_ENTRYPOINT_NOT_FOUND` 回避）

## 6. リスクと対策
- **リスク: `.sqlx/` データの不整合**  
  - 対策: 生成手順をドキュメント化し、本フェーズ完了時に `.sqlx/` のチェックサムと生成日を記録。
- **リスク: Trait 実装の移行中に `pub use` 忘れでビルドエラー**  
  - 対策: 各ステップ後に `cargo check` を挟み、小さい差分で PR を作成。
- **リスク: EventRepository のタグ処理で回帰発生**  
  - 対策: `EventManager` / `EventService` の関連処理で smoke テスト（既存ユニットテスト）を実行し、タグ挿入・取得が正しく動作するか確認。

## 7. 次アクション
1. `sqlite_repository` ディレクトリと `mod.rs`/`posts.rs` の雛形を作成し、分割用ブランチを開始する。
2. PostRepository 移行時に利用する `mapper.rs` と `queries.rs` の初版を用意し、Post領域での適用を検証。
3. `.sqlx/` 更新手順を `docs/03_implementation/database_operations.md`（仮）または既存手順書に追記するタスクを派生。
