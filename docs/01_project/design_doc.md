# kukuri Design Document

## ドキュメント情報
- **プロジェクト名**: kukuri
- **バージョン**: 1.3 (更新: MVP残タスク整理とMainline DHT方針反映)
- **最終更新日**: 2025年11月10日
- **作成者**: Grok AI (xAI)
- **目的**: 本ドキュメントは、Nostrプロトコルをデータプロトコルとして採用し、提案されたハイブリッドネットワーク手法（Cloudflare Workersを活用した準分散型ピア発見とマーケットプレイスによる機能分散）を用いて構築するP2Pベースのトピック中心ソーシャルアプリケーションの設計を記述する。Nostr互換性を確保しつつ、従来のNostrの弱点を補完するアーキテクチャを目指す。更新点として、フロントエンドにReact/TypeScript/Vite/shadcn/Zustand/Tanstack Query/Tanstack Routerを採用、P2Pネットワーク層にirohを選択、発見層をコンテナ運用可能に調整。

### 更新履歴
- **2025年11月10日 (v1.3.1)**: プロフィール Stage3（Doc/Blob + privacy）完了内容を反映し、`profile_avatar_sync` / `useProfileAvatarSync` のテストコマンドと Nightly/Docker シナリオ登録状況を追記。`roadmap.md`・`tauri_app_implementation_plan.md`・`phase5_user_flow_inventory.md` とのクロスリファレンスを更新。
- **2025年11月08日 (v1.3)**: MVP Exit Criteriaと残タスク一覧を追加し、BitTorrent Mainline DHTへの移行方針と Phase 5 の依存タスク（`refactoring_plan_2025-08-08_v3.md` / `phase5_user_flow_inventory.md`）を反映。
- **2025年07月25日 (v1.2)**: プロジェクト名の変更に伴い全体を再構成。

## 概要
kukuriは、分散型ソーシャルネットワークアプリケーションで、ユーザーがトピックベースのタイムラインを作成・共有できる。データ交換はNostrプロトコル（NIPsに基づくイベント署名と配信）を基盤とし、P2P層はBitTorrent Mainline DHT + irohでフルメッシュを確立、Cloudflare Workers/コンテナはフォールバック兼ブートストラップに限定する。これにより、P2P接続の初期化＝Mainline DHT、運用監視＝`docs/03_implementation/p2p_mainline_runbook.md` に記載のRunbookへと役割を明確化した。

2025年11月時点の設計では、トレンド/フォロー系のUXとOfflineファーストの完成度がMVPのギャップとなっている。Tauri v2をフロントエンド/バックエンドに使用し、React 18 + TypeScript + TanStack Router/Query + shadcn/ui + ZustandでUIを構築。MVP Exit Criteriaに沿って、Mainline DHTの安定化・イベントゲートウェイの抽象化・トレンド指標更新ジョブの整備までを必須要件とする。

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

## MVPスコープとExit Criteria（2025年11月11日更新）

Phase 5 で定義している MVP は「トピック/フォロー/DM を横断した最低限の体験を、Mainline DHT ベースのP2Pネットワークとオフライン同期を伴って提供し、Nightly CIとRunbookで再現可能な状態」を指す。Exit Criteriaは以下の通り。

| カテゴリ | 目的 | 達成条件 | 残タスク（2025年11月10日時点） |
| --- | --- | --- | --- |
| 体験/UX | `/trending` `/following` `/profile/$userId` `/direct-messages` の主要フローをブロッカーなしで提供 | Summary Panel/DM未読/プロフィール編集/検索/Topic作成導線が全て接続され、`phase5_user_flow_summary.md` の 3状態（稼働/改善/未実装）で「未実装」が0件になる | Stage3（Doc/Blob + privacy）は `profile_avatar_sync` + `useProfileAvatarSync` 導入で完了。2025年11月11日に Summary Panel → `trending_metrics_job` の自動監視（`prometheus-trending` + `tmp/logs/trending_metrics_job_stage4_<timestamp>.log`）を整備済み。残タスクは `TopicSelector` / `PostCard` の Vitest 再実行、DM Inbox の仮想スクロール/候補補完、検索 UI のレートリミット表示。 |
| P2P & Discovery | Mainline DHT と Gossip 経由で恒常的に接続し、イベント配送とRunbookを提供 | `docs/03_implementation/p2p_mainline_runbook.md` のRunbook完成、`bootstrap` CLI/Workersの動的更新、EventGateway抽象化の完了 | `refactoring_plan_2025-08-08_v3.md` Phase5（EventGateway/KeyManager分離）と `phase5_event_gateway_design.md` のタスク化、`kukuri-cli`のRunbook検証、`phase5_dependency_inventory_template.md` の High 優先モジュール移行 |
| データ/同期 | Offlineファースト（sync_queue/楽観更新）とトレンド指標の自動計測を統合 | `tauri_app_implementation_plan.md` Phase4 の sync_queue / offline_actions / conflict UI 完了、`trending_metrics_job` + Docker `trending-feed` シナリオが green | `OfflineService`/`useSyncManager` の競合UI、`list_trending_*` の24h集計・アーティファクト確認（`scripts/test-docker.{sh,ps1} ts --scenario trending-feed` + `tmp/logs/trending_metrics_job_stage4_<timestamp>.log` の証跡）、`phase5_user_flow_summary.md` セクション1.2/1.3の「改善中」解消 |
| オペレーション/テスト | CI/CD・NightlyでMVP体験を再現し、失敗時にRunbookで復旧可能 | `github/test.yml` の `native-test-linux` / `format-check` / `nightly trending-feed` が安定し、`scripts/test-docker.{sh,ps1}` でts/lint/rustシナリオをローカル再現可能 | `tasks/status/in_progress.md` (GitHub Actions) に記載のDocker Vitest分離とアーティファクトアップロード課題の解消、`docs/01_project/roadmap.md` にも反映するNightly KPIの定義 |

> **参照**: `phase5_user_flow_summary.md` に「MVP Exit Checklist（2025年11月09日版）」を新設し、上表4カテゴリ（UX/体験導線・P2P & Discovery・データ/同期・Ops/CI）の進捗とテスト手順を横断管理している。`phase5_user_flow_inventory.md` では同カテゴリのクロスウォークを Sec.0 に追加し、関連セクション（5.1/5.4/5.5/5.7/5.9/5.10/5.11）の執筆責任を明示した。
## システムアーキテクチャ
### 高レベル概要
- **クライアント層**: Tauri v2アプリ（Rustバックエンド + Webフロントエンド）。ユーザーがNostrイベントを作成/署名し、P2Pで共有。
- **発見層**: BitTorrent Mainline DHT + ブートストラップ CLI（`kukuri-cli`） + Cloudflare Workers（フォールバック）。本番では Mainline DHT がピア探索の一次情報を担当し、Workers/コンテナは KPI 計測と封鎖地域向けのバックアップに限定する。
- **P2Pネットワーク層**: irohを採用し、QUICベースの直接接続を実現。Mainline DHT/Workersから取得したピアリストで接続し、Nostrイベントを配信する。`phase5_dependency_inventory_template.md` で管理する P2PService Stack が抽象化を担う。
- **マーケットプレイス層**: 専門ノード（検索/サジェスト担当）。トークンエコノミーでインセンティブ化、DePINライクに分散。
- **データプロトコル**: Nostrイベント（NIP-01: 基本イベント、NIP-29: グループなど）。トピックをNostrのkindやtagで表現。

### コンポーネント詳細
1. **クライアント (Tauri App)**:
   - **フロントエンド**: React + TypeScriptを基盤に、Viteでビルド/開発環境を構築。UIコンポーネントにshadcnを採用し、状態管理にZustand、データフェッチングにTanstack Query、ルーティングにTanstack Routerを使用。モダンなUI/UXを実現し、TauriのWebViewでレンダリング。
   - **バックエンド**: RustでNostrイベント生成/署名、Workers API呼び出し、P2P接続管理（iroh統合）。
   - **機能**: Workersにピア登録、トピックサブスクライブ、マーケットプレイスクエリ。Tanstack Queryで非同期データ同期を最適化。

2. **Discovery（Mainline DHT + Workers）**:
   - **Mainline DHT**: `kukuri-cli` を bootstrap node として常駐させ、DHT ノードID/ルーティングテーブル/再接続ルールを Runbook (`docs/03_implementation/p2p_mainline_runbook.md`) に明記する。クライアントは iroh の DHT API を利用し、起動時に5ピア以上を探索することをMVP基準に設定。
   - **Cloudflare Workers/コンテナ**: DHTが塞がれた環境向けのバックアップと、トレンド/フォロー系のメトリクス収集を担当。Wrangler発のJS実装をDockerラップし、Durable ObjectsではなくPostgres/SQLiteでも再現できる形に整理する。
   - **API**: DHT優先で `announce`/`lookup` を行い、Workersは `/register`, `/lookup`, `/healthz` の軽量APIで冗長化。UIからはDiscoveryの状態を `DiscoveryStatus` カードで可視化。
   - **運用**: CLI + Workers の構成は `docs/01_project/roadmap.md` の Phase 5 KPI と連動し、ブートストラップリストの動的更新 PoC をCriticalタスクとして管理。

3. **P2Pネットワーク**:
   - irohをRustバックエンドに統合し、QUICストリームで接続。Mainline DHT で得たピア情報を優先し、FallbackとしてWorkers/コンテナのアドレス帳を利用する。
   - NAT traversal: irohの組み込み機能とWorkersシグナリングを組み合わせる。`phase5_dependency_inventory_template.md` で追跡する `P2PService Stack` リファクタにより、Mainline DHT と Gossip を抽象化したイベントバスを実装する。

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

## 実装計画（2025年11月10日更新）

| フェーズ | 範囲 | 状態 | MVP観点の残タスク |
| --- | --- | --- | --- |
| Phase 1: 認証/オンボーディング | Welcome/Login/Profile Setup/セキュアストレージ | ✅ 完了（2025-08） | - |
| Phase 2: データ連携 | タイムライン/トピックCRUD/P2Pイベント/リアルタイム | ✅ 完了 | - |
| Phase 3: 主要機能拡張 | グローバルコンポーザー、リッチ投稿、DM、トピックメッシュ | ⏳ 3.1/3.2 は完了、プロフィール編集モーダル Stage3（Doc/Blob + privacy）は 2025年11月10日にクローズ。残るMVPタスクは `TopicSelector` / `PostCard` の再テスト、DM Inbox の仮想スクロール/候補補完、`DirectMessageDialog` の多端末既読共有。3.3（ブースト/ブックマーク/カスタムリアクション）は Post-MVP。 |
| Phase 4: オフラインファースト | sync_queue/offline_actions/競合解決/Service Worker | ⏳ sync_queue・offline_actions テーブルと `useSyncManager` UI が未完。`tauri_app_implementation_plan.md` の Phase4.1〜4.3/4.4 がMVP Exit Criteria。 |
| Phase 5: アーキ/依存再構成 + Ops | EventGateway抽象化、P2PService Stack分割、トレンド指標自動集計、Mainline DHT Runbook | ⏳ Runbook Chapter10＋RelayStatus 連携は 2025年11月12日に完了（`tmp/logs/relay_status_cli_bootstrap_20251112-094500.log` を Runbook 10.3/10.4 へ反映）。残りは `phase5_event_gateway_design.md` の Gateway 実装と `trending_metrics_job` カバレッジの自動集計。 |

- **ツール/言語**: Tauri v2 (Rust/JavaScript)、React 18 + TypeScript + Vite + shadcn/ui + Zustand + TanStack Query/Router、Cloudflare Workers/Node.js コンテナ、iroh + nostr-sdk、BitTorrent Mainline DHT。
- **テスト/QA**: `pnpm vitest`（単体/結合）、`pnpm test:integration`（統合）、`cargo test`（`kukuri-tauri/src-tauri`, `kukuri-cli`）、`docker compose -f docker-compose.test.yml up --build test-runner`、Nightly（`nightly.yml`）で `trending-feed` / `native-test-linux` / `format-check` / `rust` を再現。
- **タイムライン**: 2025年11月中に Phase4/5 の MVPタスクを完了 → 12月前半でリリース準備 (Phase7) に移行 → 12月末にベータ。

## リスクと緩和策
- **リスク1: Mainline DHT/Workers の二重運用**: 緩和: DHT Runbookと`kukuri-cli`ヘルスチェック、Workersはバックアップ用途に限定し、フェイルオーバーテストをNightlyに組み込む。
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
