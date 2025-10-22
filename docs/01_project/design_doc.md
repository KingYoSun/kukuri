# kukuri Design Document

## ドキュメント情報
- **プロジェクト名**: kukuri
- **バージョン**: 1.2 (更新: プロジェクト名をkukuriに変更)
- **最終更新日**: 2025年07月25日
- **作成者**: Grok AI (xAI)
- **目的**: 本ドキュメントは、Nostrプロトコルをデータプロトコルとして採用し、提案されたハイブリッドネットワーク手法（Cloudflare Workersを活用した準分散型ピア発見とマーケットプレイスによる機能分散）を用いて構築するP2Pベースのトピック中心ソーシャルアプリケーションの設計を記述する。Nostr互換性を確保しつつ、従来のNostrの弱点を補完するアーキテクチャを目指す。更新点として、フロントエンドにReact/TypeScript/Vite/shadcn/Zustand/Tanstack Query/Tanstack Routerを採用、P2Pネットワーク層にirohを選択、発見層をコンテナ運用可能に調整。

## 概要
kukuriは、分散型ソーシャルネットワークアプリケーションで、ユーザーがトピックベースのタイムラインを作成・共有できる。データ交換はNostrプロトコル（NIPsに基づくイベント署名と配信）を基盤とし、ネットワーク層はDHTを避けたハイブリッドP2Pアプローチを採用。ピア発見をCloudflare Workersで中央集権的に管理しつつ、WorkersをOSS化してコミュニティによる分散運用を可能にする。さらに、高負荷機能（検索、サジェスト）を専門ノードのマーケットプレイスで分散し、規制耐性とスケーラビリティを向上させる。

この設計は、Nostrのシンプルさと検閲耐性を活かしつつ、提案手法でユーザー体験を強化。Tauri v2をフロントエンド/バックエンドに使用し、クロスプラットフォーム（デスクトップ/モバイル）対応を目指す。2025年のNostrエコシステムの進化（例: UX改善、P2P市場統合）を考慮し、互換性を維持。

## 目標
- **機能目標**:
  - トピックベースのタイムライン作成（例: "politics"トピックで投稿共有）。
  - Nostrイベントによるデータ配信（署名付きノート）。
  - 高負荷機能の分散（検索: トピック内コンテンツ探索、サジェスト: 関連トピック提案）。
- **非機能目標**:
  - 検閲耐性: Nostrの分散リレーモデルを拡張。
  - スケーラビリティ: 数万ユーザー対応、Workersのエッジネットワーク活用。
  - プライバシー: エンドツーエンド暗号化、クライアント側フィルタリング。
  - 規制耐性: 機能分散により当局の矛先を散らす。
  - 開発容易性: OSS WorkersとTauriで低コスト実装、コンテナ運用で柔軟性向上。
- **制約**: Cloudflare依存を最小限に（OSS化とコンテナ対応で緩和）。Nostr NIP準拠で互換性確保。

## システムアーキテクチャ
### 高レベル概要
- **クライアント層**: Tauri v2アプリ（Rustバックエンド + Webフロントエンド）。ユーザーがNostrイベントを作成/署名し、P2Pで共有。
- **発見層**: Cloudflare Workers（OSSスクリプト）。トピック-ピアレコードをKV/Durable Objectsで管理。ピアはWorkersに登録/クエリ。コンテナ運用を可能にし、Dockerイメージでローカル/セルフホスト（例: KubernetesやDocker Compose）対応。
- **P2Pネットワーク層**: irohを採用し、QUICベースの直接接続を実現。Workersから取得したピアリストで接続し、Nostrイベントを配信。
- **マーケットプレイス層**: 専門ノード（検索/サジェスト担当）。トークンエコノミーでインセンティブ化、DePINライクに分散。
- **データプロトコル**: Nostrイベント（NIP-01: 基本イベント、NIP-29: グループなど）。トピックをNostrのkindやtagで表現。

### コンポーネント詳細
1. **クライアント (Tauri App)**:
   - **フロントエンド**: React + TypeScriptを基盤に、Viteでビルド/開発環境を構築。UIコンポーネントにshadcnを採用し、状態管理にZustand、データフェッチングにTanstack Query、ルーティングにTanstack Routerを使用。モダンなUI/UXを実現し、TauriのWebViewでレンダリング。
   - **バックエンド**: RustでNostrイベント生成/署名、Workers API呼び出し、P2P接続管理（iroh統合）。
   - **機能**: Workersにピア登録、トピックサブスクライブ、マーケットプレイスクエリ。Tanstack Queryで非同期データ同期を最適化。

2. **Cloudflare Workers (Discovery Service)**:
   - OSSスクリプト（Wranglerでデプロイ可能）。コンテナ運用対応として、WorkersスクリプトをDockerコンテナにラップ（例: Node.jsベースのサーバーでエミュレートし、Cloudflare互換APIを提供）。これにより、Cloudflare外での運用（ローカルサーバーやクラウドVM）が可能。
   - **ストレージ**: KVでトピック-ピアリスト、Durable Objectsでリアルタイム更新。
   - **API**: HTTP/WSで登録/クエリ（例: /register?topic=politics&peer_id=...）。
   - **分散運用**: コミュニティが複数Workers/コンテナを立て、クライアントが複数に登録（クロス接続）。

3. **P2Pネットワーク**:
   - irohをRustバックエンドに統合し、QUICストリームで接続。
   - Workersからピアリスト取得後、irohのEndpointでP2P接続し、Nostrイベントをブロードキャスト。
   - NAT traversal: irohの組み込み機能とWorkersシグナリングを組み合わせ。

4. **マーケットプレイス (Specialized Nodes)**:
   - **ノードタイプ**: 検索ノード（トピックインデックス）、サジェストノード（AIベース推薦）。
   - **運用**: ブロックチェーン統合（トークン支払い）、Workers/コンテナでノードディスカバリ。
   - **インセンティブ**: サービス提供で報酬、悪意ノード排除のためのスラッシング。

## データフロー
1. **ユーザー登録/投稿**: クライアントでNostrキー生成、イベント作成（トピックtag付き）。Zustandで状態管理。
2. **発見**: Workers/コンテナにトピック登録、ピアリストクエリ（Tanstack Queryでキャッシュ）。
3. **P2P共有**: irohでピア接続し、イベント配信。
4. **高負荷機能**: マーケットプレイスにクエリ（例: 検索リクエスト）、結果をP2Pで統合。
5. **更新**: Workers/コンテナでリアルタイム同期（WebSocket）。

## セキュリティとプライバシー
- **認証**: Nostrの公開鍵署名でイベント検証。
- **暗号化**: irohのQUIC転送でTLS/Noiseプロトコル。
- **プライバシー**: クライアント側フィルタ（Nostr NIP準拠）、Workers/コンテナレコードはハッシュ化。
- **攻撃対策**: DDoS耐性（Cloudflare保護またはコンテナファイアウォール）、Sybil攻撃（登録制限）。
- **規制対応**: 機能分散で責任散逸、OSSでコミュニティ主導。コンテナ運用で検閲回避を強化。

## スケーラビリティとパフォーマンス
- **スケーリング**: Workers/コンテナのグローバルエッジで低遅延、マーケットノード分散で負荷分散。irohの軽量QUICで効率化。
- **ボトルネック対策**: 無料ティア超過時はコミュニティコンテナ追加。
- **パフォーマンス**: Viteの高速ビルド、Tanstack Queryの最適化フェッチ、irohの低遅延P2P、Nostrの軽量イベントで実現。2025年のNostrスケーラビリティ更新（多リレー対応）を活用。

## 実装計画
- **Phase 1 (Prototype)**: Tauri基本アプリ（React/TypeScript/Vite/shadcn/Zustand/Tanstack Query/Tanstack Router） + Workers/コンテナ OSSスクリプト + Nostrイベント統合。
- **Phase 2 (P2P/Marketplace)**: iroh接続 + マーケットプレイスプロトタイプ（トークンなし）。
- **Phase 3 (Full)**: インセンティブ追加、テストネットデプロイ。
- **ツール/言語**: Tauri v2 (Rust/JS), React/TypeScript/Vite/shadcn/Zustand/Tanstack Query/Tanstack Router, Cloudflare Workers/コンテナ (JS/Node.js), iroh (Rust), Nostrライブラリ (rust-nostr or similar)。
- **タイムライン**: 3ヶ月でMVP、6ヶ月でベータ。
- **テスト**: ユニットテスト、E2Eテスト、負荷テスト。

## リスクと緩和策
- **リスク1: Cloudflare依存**: 緩和: OSSとコンテナ運用でマルチクラウド対応拡張。
- **リスク2: Nostr互換性の逸脱**: 緩和: NIP厳守、コミュニティレビュー。
- **リスク3: 規制介入**: 緩和: 分散設計で責任分散。
- **リスク4: 採用障壁**: 緩和: シンプルUI（shadcn/Tanstack Routerで強化）、Nostrエコシステム統合。
- **リスク5: iroh統合の複雑さ**: 緩和: irohドキュメント厳守、TauriバックエンドでIPC連携。

## 参考資料
- Nostr公式: https://nostr.com/
- NIPリポジトリ: https://github.com/nostr-protocol/nips
- iroh公式: https://iroh.computer/
- Tauri v2ドキュメント: https://v2.tauri.app/
- React/shadcn/Zustand/Tanstack関連: 公式ドキュメント（2025年更新版）。
- 2025年更新: Biweekly Review, FOSDEM talksなど。

このドキュメントはイテレーション可能。将来的なフィードバックに基づき更新。