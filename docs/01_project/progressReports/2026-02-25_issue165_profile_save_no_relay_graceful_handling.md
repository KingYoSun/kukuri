# Issue #165 ローカル環境でのプロフィール保存失敗修正レポート

作成日: 2026年02月25日

## 概要

- 現象:
  - フロントで `TauriCommandError: Nostr operation failed` が表示され、プロフィール保存が失敗扱いになる。
  - バックエンドログは `Created metadata event` まで出力されるが、その後に保存成功へ進まない。
- 再現条件:
  - 外部 relay 未接続（ローカル Community Node + Tauri client のみ）でプロフィールを保存。

## 根本原因

- `EventManager::update_metadata` が `client_manager.publish_event` のエラーをそのまま返していた。
- relay 未接続時、`nostr_sdk::Client::send_event` は `no relays specified` / `not connected to any relays` 系エラーを返す。
- `publish_topic_post` には no-relay 時フォールバックが実装済みだった一方、`update_metadata` には同等処理がなく、実装が不整合だった。
- 結果として `AppError::NostrError` が `ApiResponse` で `Nostr operation failed` に正規化され、フロントでは詳細不明の失敗に見えていた。

## 実装内容

- 変更ファイル:
  - `kukuri-tauri/src-tauri/src/infrastructure/event/manager/publishing.rs`
  - `kukuri-tauri/src-tauri/tests/integration/event/manager/mod.rs`

1. no-relay 判定の共通化

- `allow_no_relay_publish(message: &str) -> bool` を追加。
- 判定条件:
  - `KUKURI_ALLOW_NO_RELAY=1`
  - `message` が `no relays specified` または `not connected to any relays` を含む

2. metadata 更新の graceful handling

- `update_metadata` で publish 失敗時に no-relay 判定を実施。
- no-relay の場合は warning ログを出し、`event.id` を返して処理継続。
- その後の P2P broadcast は従来どおり実行。

3. 挙動統一（text note）

- `publish_text_note` でも同一 helper を使うように変更し、no-relay 判定ロジックのばらつきを解消。

4. 回帰テスト追加

- `update_metadata_succeeds_without_relays_and_broadcasts_to_p2p` を統合テストへ追加。
- relay 未接続・環境変数未設定でも `update_metadata` が成功し、P2P broadcast 側へ到達することを検証。

## 実行結果

- `cargo fmt --check`: pass
- `cargo test`: pass
- `pnpm format:check / lint / type-check`:
  - ホストに Node.js / pnpm がないため Docker (`node:22-bullseye`) で実行し pass
- `gh act`:
  - `format-check`: pass
  - `native-test-linux`: pass
  - `community-node-tests`: pass
