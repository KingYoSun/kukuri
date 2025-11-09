# kukuri プロジェクトロードマップ

**作成日**: 2025年08月16日  
**最終更新**: 2025年11月10日

## プロジェクトビジョン
kukuri は Nostr イベントをベースにしたトピック中心ソーシャルクライアントとして、BitTorrent Mainline DHT + iroh を土台に完全分散なP2P体験を提供する。MVP のゴールは、トレンド/フォロー/DM/プロフィール/検索といった日常の導線を Mainline DHT 上で安定提供し、Nightly + Runbook で再現できる状態まで引き上げることにある。

## ロードマップ概要（2025年11月10日再編）
- Phase 1〜2（認証/トピック/リアルタイム）は完了済み。現在は Phase 3.3〜5 の仕上げがMVPのクリティカルパス。
- 残タスクは **UX/体験**, **P2P & Discovery**, **データ同期**, **Ops/テスト** の4トラックに整理し、Exit Criteriaを `docs/01_project/design_doc.md` と同期させる。
- 2025年11月中にMVPトラックを完了 → 12月前半でリリース準備（Phase 7）→ 12月末にベータリリースを目指す。

### MVPトラック別の残タスク
| トラック | 目的 | 主なタスク | 所属ドキュメント | 状態 |
| --- | --- | --- | --- | --- |
| UX/体験 | `/trending` `/following` `/profile/$userId` `/direct-messages` `/search` をブロッカー無しでつなぐ | `phase5_user_flow_inventory.md` 5.1〜5.7 の改善（設定モーダルのプライバシー反映、グローバルコンポーザーからのトピック作成、DM Inboxの仮想スクロール/候補補完、ユーザー検索のレートリミットUI、Summary Panelテレメトリ更新、トレンド/フォローの再実行性担保） | `phase5_user_flow_summary.md`, `tauri_app_implementation_plan.md` Phase3 | ⏳ Stage3（Doc/Blob + privacy）は 2025年11月10日に完了。`TopicSelector` / `PostCard` の Vitest 再実行、Summary Panel→`trending_metrics_job` の自動連携、DM/検索 UI の最終調整が残る。 |
| P2P & Discovery | Mainline DHT + Gossip の運用 Runbook を整え、EventGateway 経由でアプリ層へ隠蔽 | `phase5_event_gateway_design.md` の Gateway 実装、`refactoring_plan_2025-08-08_v3.md` Phase5（P2PService Stack/KeyManager分離）、`docs/03_implementation/p2p_mainline_runbook.md` の Runbook 完成、`kukuri-cli` ブートストラップリストの動的更新 PoC | `phase5_dependency_inventory_template.md`, `docs/03_implementation/p2p_mainline_runbook.md` | ⏳ 設計済/実装中 |
| データ/同期 | Offline ファースト（sync_queue/楽観更新）とトレンド指標自動集計 | `tauri_app_implementation_plan.md` Phase4（sync_queue/offline_actions/競合UI/Service Worker）、`trending_metrics_job` + `scripts/test-docker.{sh,ps1}` `--scenario trending-feed` の自動化、`list_trending_*` の24h集計と `generated_at` ミリ秒保証 | `refactoring_plan_2025-08-08_v3.md` Phase2.5/5.7, `phase5_user_flow_summary.md` 1.2 | ⏳ 実装中（ジョブ/オフライン層が未完） |
| Ops/テスト | Nightly/CIでMVP導線を再現しRunbookで復旧できる体制 | `tasks/status/in_progress.md` (GitHub Actions) のトレンドフィードDocker修正、`nightly.yml` `trending-feed` のアーティファクト権限問題切り分け、`docs/01_project/progressReports/` へのRunbookリンク、`scripts/test-docker.ps1 all` の安定化 | `docs/01_project/activeContext/tasks/status/in_progress.md`, `docs/01_project/design_doc.md` | ⏳ 継続調整 |

#### MVP Exit Checklist 連動状況（2025年11月10日）
- **UX/体験導線**: Stage2 プライバシー + Stage3 Doc/Blob（`profile_avatar_sync` + `useProfileAvatarSync`）は完了し、Nightly/Docker へ `profile-avatar-sync` シナリオを登録。`TopicSelector`/`PostCard` の Vitest 再実行（`corepack pnpm` 展開）と Summary Panel → `trending_metrics_job` の自動実行が残る。→ `phase5_user_flow_summary.md` / `phase5_user_flow_inventory.md` のクロスウォークを参照。
- **P2P & Discovery**: Chapter10 まで Runbook を拡張し RelayStatus からリンク済み。EventGateway mapper / P2P trait 化・`kukuri-cli` 動的ブートストラップ PoC が未反映。→ `phase5_event_gateway_design.md` 更新が必要。
- **データ/同期**: `list_sync_queue_items` UI と 60 秒ポーリングは完了。Doc/Blob 対応の `cache_metadata` 拡張と `trending_metrics_job` の AppState フックは進行中。→ `tauri_app_implementation_plan.md` Phase4 / `phase5_ci_path_audit.md`.
- **Ops/CI**: `pnpm` 実行環境の欠如で `TopicSelector`/`PostCard` テストがホストでは未再現。Rust テストは `./scripts/test-docker.ps1 rust -NoBuild` で迂回。→ `tasks/status/in_progress.md` GitHub Actions 節、`phase5_ci_path_audit.md` にログリンクを追記予定。

### 2025年11月: Phase 5（MVP仕上げ）
- **Week 1（完了）**: グローバルコンポーザー導線統一、DMモーダルのモックテスト整備、`direct_message_conversations` 永続化。
- **Week 2（進行中）**:
  - トレンド/フォロー Summary Panel → `trending_metrics_job` の 24h 集計と Docker シナリオ固定。
  - 設定モーダルのプライバシー設定をバックエンドへ伝播（Stage3 Doc/Blob + privacy を 2025年11月10日に完了、Runbook Chapter4/CIログへ登録済み）。
  - `EventGateway` ポート実装と `EventService` の依存置換。
- **Week 3（予定）**:
  - Offline sync_queue + conflict UI。
  - Mainline DHT Runbook / ブートストラップリスト自動更新 PoC。
  - ユーザー検索のレートリミット UI + API 拡張（`search_users`）。

### 2025年12月: Phase 7（リリース準備）
- Runbook整備（Mainline DHT 運用、`trending_metrics_job` / `nightly` パイプライン、`scripts/test-docker` 使用手順）
- セキュリティと負荷テスト (`cargo audit`, `pnpm vitest --runInCI`, `docker-compose.test.yml up --build test-runner`)
- リリースノート/ユーザー告知計画（`docs/01_project/activeContext/tasks/priority/critical.md` Phase 7項目）
- CI/CD最終調整とビルド署名 (`pnpm tauri build`, `cargo build --release` 署名確認)

### 2026年Q1: ベータ拡張
- アプリ配布チャネル（ストア/Autoupdater）
- マーケットプレイスノードのAPI公開（検索/推薦）
- プラグインエコシステム、SDK、ガバナンスモデル
- モバイル（Tauri Mobile）検証

## 技術的マイルストーン
- **完了**: 認証フロー、リアルタイム同期、トピックCRUD、P2P初期統合、グローバルコンポーザー
- **MVP残**:
  1. EventGateway + P2PService Stack の抽象化
  2. Offline sync_queue + conflict UI
  3. トレンド/フォロー Summary Panel + `trending_metrics_job`
  4. Mainline DHT Runbook / ブートストラップPoC
- **Post-MVP**: インセンティブ設計、マーケットプレイスノード、Tauri Mobile

## KPI（2025年11月版）
| 指標 | 目標 | 現状 | 次アクション |
| --- | --- | --- | --- |
| DHT接続成功率 | >95% | 約87%（`p2p_mainline_runbook.md` 実測値） | Runbook整備と `kukuri-cli` 自動再接続 |
| トレンド/フォロー応答時間 | <1.5s | 1.2〜1.6s（`list_trending_*` キャッシュ依存） | `trending_metrics_job` で24h集計 & サマリーパネルのプレフェッチ |
| テストカバレッジ（TS/Rust） | 80% | TS 71% / Rust 68% | `pnpm vitest --coverage` と `cargo tarpaulin` のCI連携 |
| Nightly 成功率 | 100%（連続5日） | 60%（Docker権限問題） | アーティファクトアップロード権限を `ACTIONS_RUNTIME_TOKEN` 不要なモードに切替 |

## 参照ドキュメント
- `docs/01_project/design_doc.md`（MVP Exit Criteria）
- `docs/01_project/refactoring_plan_2025-08-08_v3.md`（技術的負債・Phase5計画）
- `docs/01_project/activeContext/tauri_app_implementation_plan.md`（UI/UXタスク）
- `docs/01_project/activeContext/artefacts/phase5_user_flow_inventory.md`（導線棚卸し）
- `docs/03_implementation/p2p_mainline_runbook.md`（DHT運用方針）
