# kukuri 実装計画書

## ドキュメント情報
- **プロジェクト名**: kukuri
- **バージョン**: 1.0
- **作成日**: 2025年7月25日
- **最終更新**: 2025年7月26日
- **目的**: kukuriプロジェクトの段階的な実装計画と技術的詳細の定義

## 1. 実装フェーズ概要

### Phase 1: MVP（3ヶ月）
基本的な機能を実装し、動作可能な最小限のプロダクトを構築

### Phase 2: ベータ版（3ヶ月）
高度な機能追加とパフォーマンス最適化

### Phase 3: 正式リリース（継続的）
コミュニティフィードバックに基づく改善とエンタープライズ機能

## 2. Phase 1: MVP実装計画（月1〜3）

### 2.1 月1: 基盤構築

#### Week 1-2: プロジェクトセットアップ
- [x] Tauri v2プロジェクト初期化
- [x] React + TypeScript + Vite環境構築
- [x] shadcn UIコンポーネントライブラリ導入
- [ ] Zustand状態管理セットアップ
- [ ] Tanstack Query/Router設定
- [x] ESLint/Prettier設定
- [ ] Git hooks（Husky）設定

```bash
# プロジェクト初期化コマンド
# Tauriアプリケーションは既に初期化済み
cd kukuri-tauri
pnpm add @tanstack/react-query @tanstack/react-router
pnpm add zustand
pnpm dlx shadcn-ui@latest init

# Workersプロジェクトの初期化
cd ../workers/discovery
pnpm init
pnpm add -D wrangler @cloudflare/workers-types
```

#### Week 3-4: Rust基盤実装
- [ ] Nostrライブラリ統合（nostr-sdk）
- [x] 鍵管理モジュール実装
- [x] ローカルストレージ（SQLite）セットアップ
- [ ] 基本的なTauri IPC API実装

```rust
// Cargo.toml依存関係（実装済み）
[dependencies]
tauri = { version = "2", features = [] }
tauri-plugin-opener = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Nostr Protocol
nostr-sdk = "0.42"

# Database
sqlx = { version = "0.8", features = ["runtime-tokio-native-tls", "sqlite", "migrate"] }
tokio = { version = "1.44", features = ["full"] }

# P2P Networking
iroh = "0.90"
iroh-gossip = "0.90"

# Cryptography
secp256k1 = { version = "0.29", features = ["rand", "serde"] }
aes-gcm = "0.10"
sha2 = "0.10"
argon2 = "0.5"
rand = "0.8"
base64 = "0.22"

# Utilities
anyhow = "1.0"
thiserror = "2.0"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1.11", features = ["v4", "serde"] }
```

### 2.2 月2: コア機能実装

#### Week 5-6: ユーザー管理
- [ ] アカウント作成UI/ロジック
- [ ] 鍵のインポート/エクスポート機能
- [ ] プロフィール管理機能
- [ ] ログイン/ログアウト処理

```typescript
// UIコンポーネント構造
kukuri-tauri/src/
  components/
    auth/
      LoginForm.tsx
      CreateAccount.tsx
      ImportKey.tsx
    profile/
      ProfileEdit.tsx
      ProfileView.tsx
```

#### Week 7-8: トピック機能
- [ ] トピック作成/編集UI
- [ ] トピックリスト表示
- [ ] トピック参加/退出機能
- [ ] トピック内タイムライン基本実装

### 2.3 月3: P2P通信とリリース準備

#### Week 9-10: 発見層実装
- [ ] Cloudflare Workers OSS版作成
- [ ] Dockerコンテナ化
- [ ] ピア登録/検索API実装
- [ ] WebSocket接続実装

```javascript
// Workers設定例 (workers/discovery/src/index.js)
export default {
  async fetch(request, env) {
    const url = new URL(request.url);
    
    if (url.pathname === "/api/v1/peers/register") {
      return handlePeerRegistration(request, env);
    }
    if (url.pathname === "/api/v1/peers/search") {
      return handlePeerSearch(request, env);
    }
    if (url.pathname === "/api/v1/topics/discover") {
      return handleTopicDiscovery(request, env);
    }
    // ... 他のエンドポイント
  }
};
```

#### Week 11-12: iroh-gossip統合とテスト
- [ ] iroh-gossip基本統合
  - [ ] Gossipインスタンス初期化
  - [ ] トピックID管理システム
  - [ ] Nostrイベントアダプター実装
- [ ] イベント配信・購読機能
  - [ ] トピックへのブロードキャスト
  - [ ] トピックからのイベント受信
  - [ ] イベント永続化層
- [ ] 統合テスト実施
- [ ] MVP版ビルド/パッケージング

## 3. Phase 2: ベータ版実装計画（月4〜6）

### 3.1 月4: 高度な機能追加

#### Week 13-14: コンテンツ機能拡張
- [ ] 画像/動画アップロード
- [ ] リアクション機能（いいね、リポスト）
- [ ] コメント機能
- [ ] メンション/通知システム

#### Week 15-16: 検索機能実装
- [ ] ローカル検索実装
- [ ] マーケットプレイス検索ノード統合
- [ ] 検索UI/UX改善
- [ ] フィルタリング機能

### 3.2 月5: P2P機能強化

#### Week 17-18: iroh-gossip高度な最適化
- [ ] ゴシッププロトコル最適化
  - [ ] Eager/Lazyセットの動的調整
  - [ ] メッセージ重複除去
  - [ ] 帯域幅効率化
- [ ] 履歴同期機能
  - [ ] 新規ピアへの過去イベント提供
  - [ ] オフラインキャッシュ同期
- [ ] Nostrリレーブリッジ
  - [ ] 既存Nostrネットワークとの接続
  - [ ] プロトコル変換層

#### Week 19-20: 同期アルゴリズム
- [ ] 差分同期実装
- [ ] 優先度ベース同期
- [ ] オフライン対応
- [ ] 競合解決メカニズム

### 3.3 月6: 最適化とベータリリース

#### Week 21-22: パフォーマンス最適化
- [ ] 仮想スクロール実装
- [ ] 画像遅延読み込み
- [ ] WebAssembly暗号処理
- [ ] キャッシュ戦略改善

#### Week 23-24: ベータテストと改善
- [ ] クローズドベータテスト実施
- [ ] バグ修正
- [ ] UI/UX改善
- [ ] ドキュメント整備

## 4. 技術スタック詳細

### 4.1 フロントエンド
```json
{
  "dependencies": {
    "react": "^19.1.0",
    "typescript": "^5.8.3",
    "@tauri-apps/api": "^2.7.0",
    "@tanstack/react-query": "^5.83.0",
    "@tanstack/react-router": "^1.129.8",
    "zustand": "^5.0.6",
    "@radix-ui/react-*": "latest",
    "tailwindcss": "^4.1.11",
    "class-variance-authority": "^0.7.1",
    "clsx": "^2.1.1"
  }
}
```

### 4.2 バックエンド（Rust）
```toml
[dependencies]
tauri = { version = "2.7.0", features = ["api-all"] }
nostr-sdk = "0.42.0"
sqlx = { version = "0.8.6", features = ["sqlite", "runtime-tokio-native-tls"] }
argon2 = "0.5.3"
aes-gcm = "0.10.3"
iroh = "0.90.0"
iroh-blobs = "0.91.0"
iroh-gossip = "0.90.0"
tokio = { version = "1.46.1", features = ["full"] }
serde = { version = "1.0.219", features = ["derive"] }
serde_json = "1.0.141"
tracing = "0.1.41"
tracing-subscriber = "0.3.19"
```

## 5. 更新履歴

- 2025年7月26日: iroh-gossip統合に伴う実装計画の更新
- 2025年7月25日: 初版作成

### 4.3 インフラストラクチャ
- **発見層**: Cloudflare Workers / Docker
- **CI/CD**: GitHub Actions
- **パッケージング**: Tauri Bundler
- **配布**: GitHub Releases / 自動更新

## 5. 開発環境セットアップ

### 5.1 必要なツール
```bash
# Node.js環境
curl -fsSL https://get.pnpm.io/install.sh | sh -
pnpm install

# Rust環境
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup default stable

# Tauri CLI
cargo install tauri-cli

# 開発ツール
pnpm add -D @types/node vite vitest playwright
```

### 5.2 ディレクトリ構造
```
kukuri/                     # プロジェクトルート
├── kukuri-tauri/           # Tauriアプリケーション本体
│   ├── src/                # フロントエンドソース
│   │   ├── components/     # UIコンポーネント
│   │   ├── hooks/          # カスタムフック
│   │   ├── lib/            # ユーティリティ
│   │   ├── pages/          # ルートページ
│   │   ├── services/       # API・サービス層
│   │   ├── stores/         # Zustandストア
│   │   ├── types/          # TypeScript型定義
│   │   └── utils/          # ヘルパー関数
│   ├── src-tauri/          # Rustバックエンド
│   │   ├── src/
│   │   │   ├── commands/   # Tauriコマンド（IPC）
│   │   │   ├── crypto/     # 暗号処理
│   │   │   ├── db/         # データベース層
│   │   │   ├── nostr/      # Nostrプロトコル実装
│   │   │   ├── p2p/        # P2P通信（iroh統合）
│   │   │   ├── state/      # アプリケーション状態
│   │   │   └── utils/      # ユーティリティ
│   │   ├── Cargo.toml      # Rust依存関係
│   │   └── tauri.conf.json # Tauri設定
│   ├── public/             # 静的ファイル
│   ├── package.json        # Node.js依存関係
│   └── vite.config.ts      # Vite設定
├── workers/                # Cloudflare Workers（発見層）
│   ├── discovery/          # ピア発見サービス
│   │   ├── src/            # Workerソースコード
│   │   ├── wrangler.toml   # Wrangler設定
│   │   └── package.json    # 依存関係
│   └── shared/             # 共有コード
├── docker/                 # Docker関連ファイル
│   ├── discovery/          # 発見層コンテナ
│   │   └── Dockerfile      
│   └── docker-compose.yml  # 開発環境用
├── scripts/                # ユーティリティスクリプト
│   ├── install-dev-tools.sh
│   └── setup-environment.sh
├── docs/                   # プロジェクトドキュメント
│   ├── 01_project/         # プロジェクト管理
│   ├── 02_architecture/    # アーキテクチャ設計
│   ├── 05_implementation/  # 実装ガイド
│   └── nips/               # Nostr改善提案
└── README.md               # プロジェクトREADME
```

## 6. テスト戦略

### 6.1 テストレベル
- **ユニットテスト**: Vitest（TS）、cargo test（Rust）
- **統合テスト**: Tauri統合テスト
- **E2Eテスト**: Playwright
- **パフォーマンステスト**: k6/Artillery

### 6.2 テストカバレッジ目標
- ビジネスロジック: 90%以上
- UI コンポーネント: 80%以上
- API: 95%以上

## 7. リスク管理と緩和策

### 7.1 技術的リスク

| リスク | 影響度 | 発生確率 | 緩和策 |
|--------|--------|----------|--------|
| iroh統合の複雑さ | 高 | 中 | 早期プロトタイプ作成 |
| パフォーマンス問題 | 高 | 中 | 継続的プロファイリング |
| Nostr仕様変更 | 中 | 低 | 抽象化レイヤー実装 |
| P2P接続問題 | 高 | 高 | フォールバック機構 |

### 7.2 プロジェクトリスク

| リスク | 影響度 | 発生確率 | 緩和策 |
|--------|--------|----------|--------|
| スケジュール遅延 | 高 | 中 | バッファ期間確保 |
| 要件変更 | 中 | 高 | アジャイル開発 |
| リソース不足 | 高 | 低 | 早期採用計画 |

## 8. 成果物とマイルストーン

### 8.1 Phase 1成果物
- [ ] 動作可能なMVPアプリケーション
- [ ] 基本的なドキュメント
- [ ] Dockerイメージ（発見層）
- [ ] インストーラー（Windows/macOS/Linux）

### 8.2 Phase 2成果物
- [ ] フル機能のベータ版
- [ ] APIドキュメント
- [ ] ユーザーガイド
- [ ] 開発者向けドキュメント

### 8.3 マイルストーン
- **M1（月1末）**: 基盤構築完了
- **M2（月2末）**: コア機能実装完了
- **M3（月3末）**: MVP版リリース
- **M4（月4末）**: 高度機能実装完了
- **M5（月5末）**: P2P機能完成
- **M6（月6末）**: ベータ版リリース

## 9. 品質保証

### 9.1 コードレビュー
- 全PRに最低1名のレビュー必須
- 自動テスト通過必須
- コーディング規約準拠

### 9.2 継続的インテグレーション
```yaml
# .github/workflows/ci.yml
name: CI
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Run tests
        run: |
          pnpm test
          cargo test
```

## 10. 今後の展望

### Phase 3以降の機能候補
- トークンエコノミー統合
- 高度なプライバシー機能（DM暗号化）
- モバイルアプリ最適化
- エンタープライズ機能
- 分析ダッシュボード
- プラグインシステム

## 更新履歴

- 2025年7月25日: 初版作成（v1.0）