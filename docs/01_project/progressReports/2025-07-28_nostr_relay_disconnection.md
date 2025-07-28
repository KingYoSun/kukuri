# 2025年7月28日 進捗レポート - Nostrリレー接続の無効化

## 概要
既存のNostrリレーへの接続を無効化し、P2Pネットワーク（iroh/iroh-gossip）経由でのイベント配信のみを使用するように変更しました。

## 背景
- kukuriはハイブリッドP2Pアプローチを採用しており、Nostrリレーとiroh-gossipの両方を使用可能
- 開発・テスト環境では既存のNostrリレーに接続する必要がない場合がある
- P2P機能の開発に集中するため、一時的にNostrリレー接続を無効化

## 実施内容

### 1. EventManager (`src-tauri/src/modules/event/manager.rs`)
- `connect_to_default_relays()` - デフォルトリレーへの接続をコメントアウト
- `add_relay()` - カスタムリレーへの接続を無効化（ログ出力のみ）
- `start_event_stream()` - イベントストリームを無効化
- `start_health_check_loop()` - ヘルスチェックループを無効化

### 2. NostrClientManager (`src-tauri/src/modules/event/nostr_client.rs`)
- `add_relay()` - 単一リレーへの接続を無効化（モック状態「Connected」を返す）
- `add_relays()` - 複数リレーへの接続を無効化（モック状態「Connected」を返す）
- `connect()` - 全リレーへの接続を無効化
- `reconnect_failed_relays()` - 再接続処理を無効化

### 3. Tauriコマンド (`src-tauri/src/modules/event/commands.rs`)
- `initialize_nostr` - リレー接続とイベントストリーム開始をコメントアウト
- Nostrクライアントの初期化は維持（鍵の管理は必要なため）

### 4. フロントエンド
- 変更不要
- リレーステータスは空配列として扱われる
- RelayStatusコンポーネントは何も表示しない

## 技術的詳細

### 無効化の方法
- 実際のリレー接続コードをコメントアウト
- 必要な場合はモック状態を返す
- ログメッセージで無効化されていることを明示

### 影響範囲
- Nostrリレーへのイベント送信が無効
- Nostrリレーからのイベント受信が無効
- P2Pネットワーク経由のイベント配信は引き続き有効
- 鍵管理機能は影響なし

## 今後の対応
- Nostrリレー接続を再度有効化する場合は、コメントアウトした部分を元に戻すだけで対応可能
- 設定ファイルでNostrリレー接続の有効/無効を切り替えられるようにすることも検討

## 関連ファイル
- `/kukuri-tauri/src-tauri/src/modules/event/manager.rs`
- `/kukuri-tauri/src-tauri/src/modules/event/nostr_client.rs`
- `/kukuri-tauri/src-tauri/src/modules/event/commands.rs`

## 次のステップ
- P2Pネットワーク機能の開発・テストに集中
- 必要に応じてNostrリレー接続の有効/無効を設定可能にする