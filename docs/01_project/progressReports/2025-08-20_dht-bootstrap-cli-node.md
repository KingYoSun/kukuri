# DHTブートストラップCLIノード実装

**日付**: 2025年08月20日
**カテゴリー**: P2P, インフラストラクチャ
**ステータス**: 完了

## 概要

ローカル開発環境用のDHTブートストラップノードをCLI実装として構築。フロントエンド無しのRust単体で動作し、Docker環境で簡単に起動できる。

## 実装内容

### 1. 新規クレート作成

- **kukuri-cli**: スタンドアロンのCLIバイナリ
- kukuri-tauriと同じバージョンの依存関係を使用
- iroh 0.91.1とiroh-gossip 0.91.0を採用

### 2. CLIノード機能

#### ブートストラップモード
```bash
kukuri-cli bootstrap [--peers <PEERS>]
```
- DHTの初期接続ポイントとして動作
- 他のノードが接続してネットワークに参加できる

#### リレーモード
```bash
kukuri-cli relay --topics <TOPICS>
```
- トピックベースのメッセージ配信
- Gossipプロトコルを使用した効率的な配信

### 3. Docker環境

#### Dockerfile
- マルチステージビルドで最適化
- Debian bookworm-slimベースで軽量化
- 非rootユーザーで実行

#### docker-compose.yml
```yaml
services:
  bootstrap-node-1:  # ポート 11223
  bootstrap-node-2:  # ポート 11224
  relay-node-1:      # ポート 11225
  relay-node-2:      # ポート 11226 (optional)
```

### 4. 設定ファイル

#### bootstrap_nodes.json
```json
{
  "development": {
    "nodes": ["localhost:11223", "localhost:11224"]
  },
  "staging": { "nodes": [] },
  "production": { "nodes": [] }
}
```

#### bootstrap_config.rs
- 環境別の設定管理
- 動的な設定読み込み
- フォールバック機構

### 5. 起動スクリプト

#### start-bootstrap-nodes.ps1
- Windows PowerShell用
- モード選択（all, bootstrap, relay）
- プロファイル対応（default, full）

## 使用方法

### 基本的な起動

```powershell
# すべてのノードを起動
.\scripts\start-bootstrap-nodes.ps1

# ブートストラップノードのみ
.\scripts\start-bootstrap-nodes.ps1 -Mode bootstrap

# フルプロファイル（オプショナルサービス含む）
.\scripts\start-bootstrap-nodes.ps1 -Profile full
```

### Docker Composeを直接使用

```bash
cd kukuri-cli
docker-compose up -d
docker-compose logs -f
```

### kukuri-tauriから接続

```rust
// ローカル開発環境のブートストラップノード
const BOOTSTRAP_NODES: &[&str] = &[
    "localhost:11223",
    "localhost:11224",
];
```

## アーキテクチャの利点

1. **分離された実装**: フロントエンド依存なし
2. **スケーラブル**: 複数ノードを簡単に起動
3. **開発効率**: ローカルでP2Pテストが可能
4. **環境別設定**: dev/staging/prodの切り替えが容易

## 今後の改善点

1. **NodeId管理**: 固定NodeIdまたは自動生成の改善
2. **監視機能**: メトリクスとヘルスチェック
3. **永続化**: ノード状態の保存と復元
4. **クラスタリング**: 複数ホストでの分散配置

## 関連ファイル

- `/kukuri-cli/` - CLIノード実装
- `/kukuri-cli/docker-compose.yml` - Docker構成
- `/scripts/start-bootstrap-nodes.ps1` - 起動スクリプト
- `/kukuri-tauri/src-tauri/bootstrap_nodes.json` - 設定ファイル
- `/kukuri-tauri/src-tauri/src/infrastructure/p2p/bootstrap_config.rs` - 設定管理

## テスト手順

1. Dockerコンテナを起動
2. ログでNodeIDを確認
3. kukuri-tauriから接続テスト
4. トピック購読と配信の確認

## まとめ

ローカル開発環境でDHTブートストラップノードを簡単に起動できる仕組みを構築。これにより、外部サービスに依存せずにP2P機能の開発とテストが可能になった。