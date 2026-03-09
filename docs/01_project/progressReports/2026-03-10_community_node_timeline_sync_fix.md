# 公開 Community Node 配下のタイムライン同期 / 投稿伝播失敗 修正レポート

作成日: 2026年03月10日
最終更新日: 2026年03月10日

## 1. 概要

- 公開 Community Node を `https://api.kukuri.app` に設定していても、client が `ws://localhost:8082/relay` を採用して relay 接続に失敗する問題を修正した。
- 投稿同期でローカル UUID を Nostr event ID として解釈していたため `Invalid event ID format: must be 64 hex characters` になる問題を修正した。
- 起動直後同期が `EventManager not initialized` で失敗し、その後 `is_syncing` が戻らず再試行不能になる問題を合わせて修正した。

## 2. 原因

### 2.1 公開 node でも descriptor の loopback relay URL を採用していた

- `kukuri-tauri/src-tauri/src/presentation/handlers/community_node_handler.rs`
  - bootstrap descriptor の `endpoints.ws` をそのまま relay URL として採用しており、公開 `base_url` に対しても `ws://localhost:8082/relay` が優先されるケースがあった。
  - このため client 側で `Connection failed. url=ws://localhost:8082/relay error=received termination request` が発生していた。

### 2.2 未同期 event 抽出にローカル post UUID が混入していた

- `kukuri-tauri/src-tauri/src/infrastructure/database/sqlite_repository/queries.rs`
  - `SELECT_UNSYNC_EVENTS` が未同期行を広く取りすぎており、UUID 形式のローカル post まで `EventId::from_hex` に渡していた。
  - その結果 `Invalid event ID format: must be 64 hex characters` が発生していた。

### 2.3 同期失敗時に `is_syncing` が戻らなかった

- `kukuri-tauri/src-tauri/src/application/services/sync_service.rs`
  - `start_sync()` の途中で error return すると `status.is_syncing = true` のまま残り、以後の同期再試行が止まっていた。

### 2.4 startup sync が EventManager 初期化前に走り得た

- `kukuri-tauri/src-tauri/src/state.rs`
  - active account を持つ起動フローでも `EventManager` 初期化前に同期処理が走る経路があり、`failed to sync post ...: Nostr error: EventManager not initialized` を起こしていた。

## 3. 実施内容

### 3.1 relay URL 解決ロジックを公開 node 向けに補正

- `kukuri-tauri/src-tauri/src/presentation/handlers/community_node_handler.rs`
  - descriptor 由来の relay URL から host を抽出し、公開 `base_url` に対して loopback / unspecified host を返す relay URL は破棄する helper を追加した。
  - loopback base URL を使うローカル環境では従来どおり descriptor relay URL を許可し、公開環境だけを絞るようにした。
- 追加 test
  - `resolve_nostr_relay_urls_ignores_loopback_descriptor_ws_for_public_base_url`
  - `resolve_nostr_relay_urls_keeps_loopback_descriptor_ws_for_loopback_base_url`

### 3.2 未同期 event 抽出を 64 桁 hex に限定

- `kukuri-tauri/src-tauri/src/infrastructure/database/sqlite_repository/queries.rs`
  - `length(event_id) = 64` と hex 文字だけを許可する条件を追加した。
- `kukuri-tauri/src-tauri/src/infrastructure/database/sqlite_repository/events.rs`
  - `get_unsync_events_skips_local_uuid_posts` を追加し、UUID post が未同期 event 一覧に混ざらないことを確認した。

### 3.3 同期失敗後の状態固着を解消

- `kukuri-tauri/src-tauri/src/application/services/sync_service.rs`
  - `start_sync()` を result 収束型に整理し、成功・失敗にかかわらず最後に `is_syncing = false` へ戻すよう変更した。
  - 成功時のみ `last_sync` と pending counters を更新するよう整理した。
- 追加 test
  - `start_sync_resets_is_syncing_after_failure`

### 3.4 startup sync 前に EventManager を初期化

- `kukuri-tauri/src-tauri/src/state.rs`
  - `event_manager.set_event_topic_store(...)` の直後に、active account があれば `initialize_with_keypair()` を試行するよう変更した。
  - 初期化失敗時は `warn` に留め、app 起動は継続する。

## 4. 検証

### 4.1 Rust / Docker

- `./scripts/test-docker.ps1 rust`: pass

### 4.2 Required jobs

- `gh act --workflows .github/workflows/test.yml --job format-check`: pass
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`: pass
- `gh act --workflows .github/workflows/test.yml --job community-node-tests`: pass

## 5. タスク管理反映

- `docs/01_project/activeContext/tasks/completed/2026-03-10.md`
  - 完了内容と検証結果を追記した。
- `docs/01_project/activeContext/tasks/status/in_progress.md`
  - 完了したコード修正は未完了一覧から切り離し、残る live-path 確認のみを未解決項目として記録した。

## 6. 実機検証差分（2026年03月10日）

- `https://api.kukuri.app` を設定したデスクトップ実機で、peer 間接続、投稿伝播、realtime mode での timeline 自動更新は確認できた。
- 一方、実機ログでは以下の relay warning が継続している。
  - `2026-03-09T16:17:13.991662Z ERROR nostr_relay_pool::relay::inner: Connection failed. url=wss://api.kukuri.app/relay error=HTTP error: 404 Not Found`
  - `2026-03-09T16:17:15.424673Z WARN kukuri_lib::presentation::commands::community_node_commands: Failed to apply community node Nostr relay configuration error=No configured Nostr relay connected within 3s: wss://api.kukuri.app/relay relay_count=1 reason="authenticate"`
- つまり、前回まで未確認だった timeline / post 伝播の live-path 自体は通っているが、relay 設定警告と profile 伝播は別途残っている。

## 7. 残課題

- `wss://api.kukuri.app/relay` の 404 と `No configured Nostr relay connected within 3s` warning の切り分け
- profile update を伴わない通常投稿シナリオで、相手側 timeline/thread の display name / avatar が反映されない問題
- 上記ユースケースを担保できていない Community Node profile propagation E2E の補強

関連レポート: `docs/01_project/progressReports/2026-03-10_community_node_profile_propagation_gap.md`
