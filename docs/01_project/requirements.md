# kukuri 要件定義書

## ドキュメント情報
- **プロジェクト名**: kukuri
- **バージョン**: 1.0
- **作成日**: 2025年7月25日
- **目的**: Nostrプロトコルベースの分散型トピック中心ソーシャルアプリケーションの要件定義

## 1. 機能要件

### 1.1 ユーザー管理
- **FR-001**: ユーザーはNostrプロトコル準拠の鍵ペア（公開鍵/秘密鍵）を生成できる
- **FR-002**: ユーザーは既存の秘密鍵をインポートできる
- **FR-003**: ユーザーはプロフィール情報（名前、アバター、説明）を設定・更新できる
- **FR-004**: ユーザーは秘密鍵を安全にバックアップ・復元できる

### 1.2 トピック管理
- **FR-010**: ユーザーは新しいトピックを作成できる
- **FR-011**: ユーザーはトピックをフォロー/アンフォローできる
- **FR-012**: ユーザーはトピック内で投稿（Nostrイベント）を作成できる
- **FR-013**: ユーザーはトピックの一覧を検索・閲覧できる
- **FR-014**: トピックはタグやカテゴリーで分類される

### 1.3 コンテンツ共有
- **FR-020**: ユーザーはテキスト投稿を作成・編集・削除できる
- **FR-021**: ユーザーは画像・動画を投稿に添付できる
- **FR-022**: ユーザーは他のユーザーの投稿にリアクション（いいね、リポスト）できる
- **FR-023**: ユーザーは投稿にコメントできる
- **FR-024**: 投稿はNostrイベントとして署名・配信される

### 1.4 P2P通信
- **FR-030**: クライアントはirohを使用してP2P接続を確立できる
- **FR-031**: クライアントはピア発見サービスからピアリストを取得できる
- **FR-032**: クライアントは複数のピアとイベントを同期できる
- **FR-033**: クライアントはオフライン時のイベントを再同期できる

### 1.5 検索機能
- **FR-040**: ユーザーはトピック内のコンテンツを検索できる
- **FR-041**: ユーザーはユーザー名で他のユーザーを検索できる
- **FR-042**: 検索はマーケットプレイスの専門ノードを利用できる

### 1.6 サジェスト機能
- **FR-050**: システムは関連トピックを提案できる
- **FR-051**: システムはフォローすべきユーザーを提案できる
- **FR-052**: サジェストはマーケットプレイスの専門ノードを利用できる

## 2. 非機能要件

### 2.1 パフォーマンス
- **NFR-001**: アプリケーションの起動時間は3秒以内
- **NFR-002**: トピックタイムラインの読み込みは1秒以内
- **NFR-003**: P2P接続の確立は5秒以内
- **NFR-004**: 100件の投稿表示で60fps以上のスクロール性能

### 2.2 スケーラビリティ
- **NFR-010**: システムは10万人以上のアクティブユーザーをサポート
- **NFR-011**: 各トピックは1万件以上の投稿を処理可能
- **NFR-012**: クライアントは100以上のピアと同時接続可能

### 2.3 セキュリティ
- **NFR-020**: 全ての通信はTLS/Noiseプロトコルで暗号化
- **NFR-021**: 秘密鍵はローカルで安全に保管（暗号化）
- **NFR-022**: Nostrイベントの署名検証は必須
- **NFR-023**: DDoS攻撃への耐性

### 2.4 可用性
- **NFR-030**: オフラインモードでのローカルコンテンツ閲覧
- **NFR-031**: ピア発見サービスの冗長性（複数のWorkers/コンテナ）
- **NFR-032**: 24/7の可用性（99.9%以上のアップタイム目標）

### 2.5 互換性
- **NFR-040**: Nostr NIPs（NIP-01、NIP-29等）への準拠
- **NFR-041**: 既存のNostrクライアントとの相互運用性
- **NFR-042**: クロスプラットフォーム対応（Windows、macOS、Linux、iOS、Android）

### 2.6 ユーザビリティ
- **NFR-050**: 直感的なUI/UX（shadcn UIコンポーネント使用）
- **NFR-051**: レスポンシブデザイン
- **NFR-052**: アクセシビリティ対応（WCAG 2.1 AA準拠）
- **NFR-053**: 多言語対応（日本語、英語）

## 3. ユーザーストーリー

### 3.1 新規ユーザー
```
As a 新規ユーザー
I want to アカウントを作成してトピックに参加する
So that コミュニティと交流できる
```

### 3.2 コンテンツクリエイター
```
As a コンテンツクリエイター
I want to 特定のトピックで情報を発信する
So that 興味を持つ人々に届けられる
```

### 3.3 情報収集者
```
As a 情報収集者
I want to 複数のトピックから効率的に情報を収集する
So that 最新の情報を把握できる
```

### 3.4 プライバシー重視ユーザー
```
As a プライバシー重視ユーザー
I want to 検閲耐性のあるプラットフォームで発信する
So that 自由に意見を表明できる
```

## 4. 制約条件

### 4.1 技術的制約
- Tauri v2フレームワークの使用
- Rustバックエンド + React/TypeScriptフロントエンド
- irohによるP2P通信
- Cloudflare Workers/コンテナによるピア発見

### 4.2 法的制約
- 各国の法規制への準拠
- プライバシー保護規則（GDPR等）への対応
- コンテンツモデレーションポリシーの策定

### 4.3 リソース制約
- 初期開発チーム: 3-5名
- 開発期間: 6ヶ月（MVP: 3ヶ月）
- 予算: 要確定

## 5. 優先順位

### Phase 1（MVP - 3ヶ月）
1. 基本的なユーザー管理（FR-001〜004）
2. トピック作成・参加（FR-010〜012）
3. テキスト投稿機能（FR-020、024）
4. 基本的なP2P通信（FR-030〜032）

### Phase 2（ベータ - 6ヶ月）
1. 完全なコンテンツ共有機能（FR-021〜023）
2. 検索機能（FR-040〜042）
3. サジェスト機能（FR-050〜052）
4. パフォーマンス最適化

### Phase 3（正式リリース後）
1. 高度なマーケットプレイス機能
2. トークンエコノミー統合
3. エンタープライズ機能

## 6. 成功基準

### 6.1 定量的指標
- 日次アクティブユーザー数: 10,000人以上
- 月間投稿数: 100,000件以上
- システム稼働率: 99.9%以上
- ユーザー満足度: 4.0/5.0以上

### 6.2 定性的指標
- ユーザーコミュニティの活性化
- 検閲耐性の実証
- Nostrエコシステムへの貢献
- 開発者コミュニティの形成

## 7. リスクと前提条件

### 7.1 リスク
- Nostrプロトコルの変更による互換性問題
- P2P通信の技術的課題
- 規制当局からの圧力
- ユーザー採用の遅れ

### 7.2 前提条件
- Nostrプロトコルの継続的な発展
- P2P技術の成熟
- 暗号資産/Web3技術への理解
- オープンソースコミュニティの協力

## 8. 用語定義

- **Nostr**: Notes and Other Stuff Transmitted by Relays - 分散型ソーシャルプロトコル
- **NIP**: Nostr Implementation Possibilities - Nostrの仕様提案
- **iroh**: P2P通信ライブラリ（QUIC/Noise使用）
- **トピック**: ユーザーが作成・参加できるテーマ別のコミュニティ
- **イベント**: Nostrプロトコルにおける署名付きメッセージ
- **ピア**: P2Pネットワークの参加ノード

## 更新履歴

- 2025年7月25日: 初版作成（v1.0）