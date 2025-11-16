# Kukuri CLI Node

DHTブートストラップノードとリレーノードのCLI実装。ローカル開発環境用のP2Pインフラストラクチャを提供します。

## 機能

- **ブートストラップノード**: DHT参加用の初期接続ポイント
- **リレーノード**: トピックベースのメッセージ配信
- **Docker対応**: 簡単なデプロイと管理

## クイックスタート

### Dockerを使用（推奨）

```bash
# すべてのノードを起動
docker-compose up -d

# ログを確認
docker-compose logs -f

# 特定のサービスのみ起動
docker-compose up -d bootstrap-node-1 bootstrap-node-2

# すべてのノード（追加プロファイル含む）を起動
docker-compose --profile full up -d
```

### ローカルビルド

```bash
# ビルド
cargo build --release

# ブートストラップノードとして実行
./target/release/kukuri-cli bootstrap

# リレーノードとして実行
./target/release/kukuri-cli relay --topics kukuri,test

# カスタムポートで実行
./target/release/kukuri-cli bootstrap --bind 0.0.0.0:9999
```

## コマンドラインオプション

### グローバルオプション

- `-b, --bind <ADDRESS>`: バインドアドレス（デフォルト: 0.0.0.0:11223）
- `-l, --log-level <LEVEL>`: ログレベル (trace, debug, info, warn, error)
- `--json-logs`: JSON形式でログ出力

### ブートストラップモード

```bash
kukuri-cli bootstrap [OPTIONS]
```

オプション:
- `--peers <PEERS>`: 接続する他のブートストラップノード（形式: node_id@host:port）

### リレーモード

```bash
kukuri-cli relay [OPTIONS]
```

オプション:
- `--topics <TOPICS>`: 購読するトピック（カンマ区切り、デフォルト: kukuri。指定値は内部的に `kukuri:` 名前空間へ変換されます）

## 環境変数

- `BIND_ADDRESS`: バインドアドレス
- `LOG_LEVEL`: ログレベル
- `JSON_LOGS`: JSON形式のログ出力を有効化

## ネットワーク構成

デフォルトのDocker構成:

| サービス | ポート | 説明 |
|---------|-------|------|
| bootstrap-node-1 | 11223 | プライマリブートストラップ |
| bootstrap-node-2 | 11224 | セカンダリブートストラップ |
| relay-node-1 | 11225 | リレーノード（開発用） |
| relay-node-2 | 11226 | リレーノード（本番用、オプション） |

## kukuri-tauriとの統合

kukuri-tauriアプリケーションから接続する場合:

1. Dockerコンテナを起動
2. 以下のアドレスを使用してDHTに参加:
   - `localhost:11223` (bootstrap-node-1)
   - `localhost:11224` (bootstrap-node-2)

### 設定例

```rust
// kukuri-tauri内での使用例
const BOOTSTRAP_NODES: &[&str] = &[
    "localhost:11223",
    "localhost:11224",
];
```

## 開発

### ビルド要件

- Rust 1.83+
- Docker & Docker Compose（オプション）

### テスト

```bash
# ユニットテスト
cargo test

# 統合テスト（Dockerコンテナを使用）
./scripts/test-integration.sh
```

## トラブルシューティング

### ポートが使用中の場合

```bash
# 使用中のポートを確認
netstat -an | grep 11223

# docker-compose.ymlでポートを変更
```

### ノードIDの確認

起動時にログに出力されます:
```
Node ID: 12D3KooWxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
```

### 接続できない場合

1. ファイアウォール設定を確認
2. Dockerネットワークが正しく作成されているか確認
3. ログレベルをdebugに設定して詳細を確認

## ライセンス

MIT
