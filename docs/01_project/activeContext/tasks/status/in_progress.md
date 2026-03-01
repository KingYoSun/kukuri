[title] 作業中タスク（in_progress）

最終更新日: 2026年03月01日

## 方針（2025年09月15日 更新）

- 当面は Nostr リレーとは接続せず、P2P（iroh + iroh-gossip + DHT）で完結した体験を優先。
- 全イベントは NIPs 準拠のスキーマに沿って取り扱う。
- Tauri v2 では E2E が困難なため、層別テスト（単体・結合/契約＋スモーク最小限）でカバレッジを確保。

## 現在のタスク
- P2Pトピック同期の不具合調査と修正（bootstrap反映、受信投稿のリアルタイム反映、プロフィール表示改善）

### 進捗メモ（2026年03月01日）
- `p2p://message/raw` のみを購読し、kind/tags 欠落によるスレッド判定崩れを抑制。
- TextNote(kind=1) の受信をトピック投稿として扱えるようにし、UI差分反映漏れを軽減。
- 投稿表示の author 解決を `postMapper` 側にも追加し、プロフィール取得の再試行（TTL付き）を導入。
- Rust `UserRepository` に `profiles` テーブルのフォールバック参照を追加し、`get_user_by_pubkey` / `get_user` でプロフィール復元可能にした。
