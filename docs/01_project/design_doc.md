# kukuri Design Document

## ドキュメント情報
- **プロジェクト名**: kukuri
- **バージョン**: 1.3.3 (Stage4: プロフィール Service Worker / Offline sync / trending metrics artefact 反映 + Discovery の DHT 一本化)
- **最終更新日**: 2025年11月21日
- **作成者**: Grok AI (xAI)
- **目的**: 本ドキュメントは、Nostrプロトコルをデータプロトコルとして採用し、Mainline DHT + iroh ベースの純分散 P2P ネットワークで構築するトピック中心ソーシャルアプリケーションの設計を記述する。Nostr互換性を確保しつつ、従来のNostrの弱点を補完するアーキテクチャを目指す。更新点として、フロントエンドにReact/TypeScript/Vite/shadcn/Zustand/Tanstack Query/Tanstack Routerを採用し、P2Pネットワーク層にirohを選択した。

### 更新履歴
- **2025年11月21日 (v1.3.3)**: Discovery を Mainline DHT + `kukuri-cli` ブートストラップに一本化。MVP セクションとロードマップ、README/requirements の記述を DHT ベース前提に整理し、最新の Nightly artefact 参照先を確認。
- **2025年11月13日 (v1.3.2)**: Stage4（プロフィール avatar Sync Service Worker + Offline ログ、Offline sync_queue Doc/Blob 拡張、`trending_metrics_job` Prometheus 監視、`nightly.topic-create` / `nightly.post-delete-cache` artefact）を反映。`design_doc.md` / `roadmap.md` / `tauri_app_implementation_plan.md` / `phase5_user_flow_summary.md` / `phase5_ci_path_audit.md` の日付・ログ連携を更新し、残タスクを DM 既読共有 + `/search` レートリミット UI + GitHub Actions 調整へ再整理。
- **2025年11月10日 (v1.3.1)**: プロフィール Stage3（Doc/Blob + privacy）完了内容を反映し、`profile_avatar_sync` / `useProfileAvatarSync` のテストコマンドと Nightly/Docker シナリオ登録状況を追記。`roadmap.md`・`tauri_app_implementation_plan.md`・`phase5_user_flow_inventory.md` とのクロスリファレンスを更新。
- **2025年11月08日 (v1.3)**: MVP Exit Criteriaと残タスク一覧を追加し、BitTorrent Mainline DHTへの移行方針と Phase 5 の依存タスク（`docs/01_project/deprecated/refactoring_plan_2025-08-08_v3.md`（アーカイブ済み） / `phase5_user_flow_inventory.md`）を反映。
- **2025年07月25日 (v1.2)**: プロジェクト名の変更に伴い全体を再構成。

## 概要
kukuriは、分散型ソーシャルネットワークアプリケーションで、ユーザーがトピックベースのタイムラインを作成・共有できる。データ交換はNostrプロトコル（NIPsに基づくイベント署名と配信）を基盤とし、P2P層はBitTorrent Mainline DHT + irohでフルメッシュを確立する。Discovery は `kukuri-cli` が配布するブートストラップリストと Mainline DHT のルーティングに限定する。これにより、P2P接続の初期化＝Mainline DHT、運用監視＝`docs/03_implementation/p2p_mainline_runbook.md` に記載のRunbookへと役割を明確化した。

2025年11月時点の設計では、トレンド/フォロー系のUXとOfflineファーストの完成度がMVPのギャップとなっている。Tauri v2をフロントエンド/バックエンドに使用し、React 18 + TypeScript + TanStack Router/Query + shadcn/ui + ZustandでUIを構築。MVP Exit Criteriaに沿って、Mainline DHTの安定化・イベントゲートウェイの抽象化・トレンド指標更新ジョブの整備までを必須要件とする。

## 目標
- **機能目標**:
  - トピックベースのタイムライン作成（例: "politics"トピックで投稿共有）。
  - Nostrイベントによるデータ配信（署名付きノート）。
  - 高負荷機能の分散（検索: トピック内コンテンツ探索、サジェスト: 関連トピック提案）。
- **非機能目標**:
  - 検閲耐性: Nostrの分散リレーモデルを拡張。
  - スケーラビリティ: 数万ユーザー対応、Mainline DHT ブートストラップリストの複数化と `kukuri-cli` エクスポートで冗長化。
  - プライバシー: エンドツーエンド暗号化、クライアント側フィルタリング。
  - 規制耐性: 機能分散により当局の矛先を散らす。
  - 開発容易性: Tauri + `kukuri-cli` による分散ブートストラップで低コスト実装、Runbook を用いた運用手順の簡素化。
- **制約**: Discovery は Mainline DHT と `kukuri-cli` ブートストラップで完結させる。Nostr NIP準拠で互換性確保。

## MVPスコープとExit Criteria（2025年11月20日更新）

Phase 5 で定義している MVP は「トピック/フォロー/DM を横断した最低限の体験を、Mainline DHT ベースのP2Pネットワークとオフライン同期を伴って提供し、Nightly CIとRunbookで再現可能な状態」を指す。以下の Exit Criteria はすべて充足した。

| カテゴリ | 目的 | 達成条件 | 結果（2025年11月20日時点） |
| --- | --- | --- | --- |
| 体験/UX | `/trending` `/following` `/profile/$userId` `/direct-messages` の主要フローをブロッカーなしで提供 | Summary Panel/DM未読/プロフィール編集/検索/Topic作成導線が全て接続され、`phase5_user_flow_summary.md` の 3状態（稼働/改善/未実装）で「未実装」が0件になる | ✅ Stage4 完了後に DM 既読 contract テスト・`/search` レートリミット UI を反映。`test-results/{trending-feed,direct-message,user-search-pagination}` と `tmp/logs/vitest_direct_message_*.log` で Nightly 再現性を担保。 |
| P2P & Discovery | Mainline DHT と Gossip 経由で恒常的に接続し、イベント配送とRunbookを提供 | `docs/03_implementation/p2p_mainline_runbook.md` のRunbook完成、`bootstrap` CLI の動的更新、EventGateway抽象化の完了 | ✅ EventGateway ポート実装と `P2PStack` trait 化を完了（`tmp/logs/cargo-test-kukuri-tauri_di_20251113.log` 等）。Runbook Chapter10・CLI PoC・`p2p_metrics_export` を Nightly `integration` ジョブと同期。 |
| データ/同期 | Offlineファースト（sync_queue/楽観更新）とトレンド指標の自動計測を統合 | `tauri_app_implementation_plan.md` Phase4 の sync_queue / offline_actions / conflict UI 完了、`trending_metrics_job` + Docker `trending-feed` シナリオが green | ✅ `trending_metrics_job` / `offline-sync` / `profile-avatar-sync` artefact を固定し、`offline_metrics.rs` で再送メトリクスを収集。`test-results/trending-feed/{reports,prometheus,metrics}`・`test-results/offline-sync/{category}` から Runbook Chapter5 で追跡可能。 |
| オペレーション/テスト | CI/CD・NightlyでMVP体験を再現し、失敗時にRunbookで復旧可能 | `github/test.yml` の `native-test-linux` / `format-check` / `nightly trending-feed` が安定し、`scripts/test-docker.{sh,ps1}` でts/lint/rustシナリオをローカル再現可能 | ✅ `nightly.topic-create/post-delete/profile-avatar-sync/trending-feed/user-search-pagination/sync-status-indicator` を揃え、`gh act --workflows .github/workflows/test.yml --job {format-check,native-test-linux}` のログを `.act-artifacts/` に保存。GitHub Actions のキャッシュ・権限対策は `phase5_ci_path_audit.md` に明記。 |

> **参照**: `phase5_user_flow_summary.md` に「MVP Exit Checklist（2025年11月09日版）」を新設し、上表4カテゴリ（UX/体験導線・P2P & Discovery・データ/同期・Ops/CI）の進捗とテスト手順を横断管理している。`phase5_user_flow_inventory.md` では同カテゴリのクロスウォークを Sec.0 に追加し、関連セクション（5.1/5.4/5.5/5.7/5.9/5.10/5.11）の執筆責任を明示した。
## システムアーキテクチャ
### 高レベル概要
- **クライアント層**: Tauri v2アプリ（Rustバックエンド + Webフロントエンド）。ユーザーがNostrイベントを作成/署名し、P2Pで共有。
- **発見層**: BitTorrent Mainline DHT + ブートストラップ CLI（`kukuri-cli`）。本番では Mainline DHT がピア探索の一次情報を担当し、CLI が配布するブートストラップリストと Runbook で運用。
- **P2Pネットワーク層**: irohを採用し、QUICベースの直接接続を実現。Mainline DHT から取得したピアリストで接続し、Nostrイベントを配信する。`phase5_dependency_inventory_template.md` で管理する P2PService Stack が抽象化を担う。
- **マーケットプレイス層**: 専門ノード（検索/サジェスト担当）。トークンエコノミーでインセンティブ化、DePINライクに分散。
- **データプロトコル**: Nostrイベント（NIP-01: 基本イベント、NIP-29: グループなど）。トピックをNostrのkindやtagで表現。

### コンポーネント詳細
1. **クライアント (Tauri App)**:
   - **フロントエンド**: React + TypeScriptを基盤に、Viteでビルド/開発環境を構築。UIコンポーネントにshadcnを採用し、状態管理にZustand、データフェッチングにTanstack Query、ルーティングにTanstack Routerを使用。モダンなUI/UXを実現し、TauriのWebViewでレンダリング。
   - **バックエンド**: RustでNostrイベント生成/署名、Mainline DHT への announce/lookup、P2P接続管理（iroh統合）。
   - **機能**: `kukuri-cli` ブートストラップの適用、トピックサブスクライブ、マーケットプレイスクエリ。Tanstack Queryで非同期データ同期を最適化。

2. **Discovery（Mainline DHT + CLIブートストラップ）**:
   - **Mainline DHT**: `kukuri-cli` を bootstrap node として常駐させ、DHT ノードID/ルーティングテーブル/再接続ルールを Runbook (`docs/03_implementation/p2p_mainline_runbook.md`) に明記する。クライアントは iroh の DHT API を利用し、起動時に5ピア以上を探索することをMVP基準に設定。
   - **ブートストラップ管理**: `kukuri-cli --export-path` で生成した複数ノードのリストをアプリに取り込み、UI から `apply_cli_bootstrap_nodes` で適用できるようにする。Docker かベアメタルで運用可能な構成に統一。
   - **API/設定**: Discovery は DHT の `announce`/`lookup` のみを実行し、UI 上は `DiscoveryStatus` カードでピア探索の到達状況と Runbook へのリンクを提示する。`docs/01_project/roadmap.md` で CLI PoC・Runbook 連携の KPI を追跡。
   - **運用**: ブートストラップリストの動的更新 PoC と Runbook Chapter10 を同期させ、`scripts/test-docker.ps1 integration` と `.act-artifacts/` のログで接続性を確認する。

3. **P2Pネットワーク**:
   - irohをRustバックエンドに統合し、QUICストリームで接続。Mainline DHT で得たピア情報を優先し、ブートストラップリスト（`kukuri-cli` エクスポート）を補助的に利用する。
   - NAT traversal: irohの組み込み機能を用い、必要に応じてブートストラップノード経由で再探索する。`phase5_dependency_inventory_template.md` で追跡する `P2PService Stack` リファクタにより、Mainline DHT と Gossip を抽象化したイベントバスを実装する。

4. **マーケットプレイス (Specialized Nodes)**:
   - **ノードタイプ**: 検索ノード（トピックインデックス）、サジェストノード（AIベース推薦）。
   - **運用**: ブロックチェーン統合（トークン支払い）、Mainline DHT/ブートストラップリストでノードディスカバリ。
   - **インセンティブ**: サービス提供で報酬、悪意ノード排除のためのスラッシング。

## データフロー
1. **ユーザー登録/投稿**: クライアントでNostrキー生成、イベント作成（トピックtag付き）。Zustandで状態管理。
2. **発見**: Mainline DHT に `announce`/`lookup` し、`kukuri-cli` から配布されたブートストラップリストをもとにピアリストをキャッシュ。
3. **P2P共有**: irohでピア接続し、イベント配信。
4. **高負荷機能**: マーケットプレイスにクエリ（例: 検索リクエスト）、結果をP2Pで統合。
5. **更新**: Mainline DHT の再探索とブートストラップリストの再適用でリアルタイム同期を維持。

## セキュリティとプライバシー
- **認証**: Nostrの公開鍵署名でイベント検証。
- **暗号化**: irohのQUIC転送でTLS/Noiseプロトコル。
- **プライバシー**: クライアント側フィルタ（Nostr NIP準拠）、ブートストラップリストは最小限のメタデータのみ保持。
- **攻撃対策**: DDoS耐性（ブートストラップノードの防火壁・ピア数制限）、Sybil攻撃（登録制限）。
- **規制対応**: 機能分散で責任散逸、OSSでコミュニティ主導。コンテナ運用で検閲回避を強化。

## スケーラビリティとパフォーマンス
- **スケーリング**: Mainline DHT と複数ブートストラップノードでピア探索を冗長化し、マーケットノード分散で負荷分散。irohの軽量QUICで効率化。
- **ボトルネック対策**: ブートストラップノードの追加とノード設定の自動ローテーションで輻輳を回避。
- **パフォーマンス**: Viteの高速ビルド、Tanstack Queryの最適化フェッチ、irohの低遅延P2P、Nostrの軽量イベントで実現。2025年のNostrスケーラビリティ更新（多リレー対応）を活用。

## 実装計画（2025年11月10日更新）

| フェーズ | 範囲 | 状態 | MVP観点の残タスク |
| --- | --- | --- | --- |
| Phase 1: 認証/オンボーディング | Welcome/Login/Profile Setup/セキュアストレージ | ✅ 完了（2025-08） | - |
| Phase 2: データ連携 | タイムライン/トピックCRUD/P2Pイベント/リアルタイム | ✅ 完了 | - |
| Phase 3: 主要機能拡張 | グローバルコンポーザー、リッチ投稿、DM、トピックメッシュ | ⏳ 3.1/3.2 は完了し、プロフィール編集モーダル Stage4（Service Worker + Offline ログ）は 2025年11月12日にクローズ。`TopicSelector` / `PostCard` / `post-delete-cache` / `topic-create` の再テストは `tmp/logs/topic_create_host_20251112-231141.log` 等で完了済み。残るMVPタスクは DM Inbox の多端末既読共有 + contract テスト、`DirectMessageDialog` 再送ログと `/search` レートリミット UI。3.3（ブースト/ブックマーク/カスタムリアクション）は Post-MVP。 |
| Phase 4: オフラインファースト | sync_queue/offline_actions/競合解決/Service Worker | ⏳ sync_queue・offline_actions テーブルと `useSyncManager` UI が未完。`tauri_app_implementation_plan.md` の Phase4.1〜4.3/4.4 がMVP Exit Criteria。 |
| Phase 5: アーキ/依存再構成 + Ops | EventGateway抽象化、P2PService Stack分割、トレンド指標自動集計、Mainline DHT Runbook | ⏳ Runbook Chapter10＋RelayStatus 連携は 2025年11月12日に完了（`tmp/logs/relay_status_cli_bootstrap_20251112-094500.log` を Runbook 10.3/10.4 へ反映）。残りは `phase5_event_gateway_design.md` の Gateway 実装と `trending_metrics_job` カバレッジの自動集計。 |

- **ツール/言語**: Tauri v2 (Rust/JavaScript)、React 18 + TypeScript + Vite + shadcn/ui + Zustand + TanStack Query/Router、iroh + nostr-sdk、BitTorrent Mainline DHT、`kukuri-cli`（ブートストラップ管理）。
- **テスト/QA**: `pnpm vitest`（単体/結合）、`pnpm test:integration`（統合）、`cargo test`（`kukuri-tauri/src-tauri`, `kukuri-cli`）、`docker compose -f docker-compose.test.yml up --build test-runner`、Nightly（`nightly.yml`）で `trending-feed` / `native-test-linux` / `format-check` / `rust` を再現。
- **タイムライン**: 2025年11月中に Phase4/5 の MVPタスクを完了 → 12月前半でリリース準備 (Phase7) に移行 → 12月末にベータ。

## リスクと緩和策
- **リスク1: Mainline DHT ブートストラップの単一障害点化**: 緩和: DHT Runbookと`kukuri-cli`ヘルスチェックで複数ノードを常時運用し、フェイルオーバーテストをNightlyに組み込む。
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
