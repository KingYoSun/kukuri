# kukuri プロジェクトロードマップ

**作成日**: 2025年08月16日  
**最終更新**: 2025年11月13日

## プロジェクトビジョン
kukuri は Nostr イベントをベースにしたトピック中心ソーシャルクライアントとして、BitTorrent Mainline DHT + iroh を土台に完全分散なP2P体験を提供する。MVP のゴールは、トレンド/フォロー/DM/プロフィール/検索といった日常の導線を Mainline DHT 上で安定提供し、Nightly + Runbook で再現できる状態まで引き上げることにある。

## ロードマップ概要（2025年11月13日再編）
- Phase 1〜2（認証/トピック/リアルタイム）は完了済み。現在は Phase 3.3〜5 の仕上げがMVPのクリティカルパス。
- 残タスクは **UX/体験**, **P2P & Discovery**, **データ同期**, **Ops/テスト** の4トラックに整理し、Exit Criteriaを `docs/01_project/design_doc.md` と同期させる。
- 2025年11月中にMVPトラックを完了 → 12月前半でリリース準備（Phase 7）→ 12月末にベータリリースを目指す。

### MVPトラック別の残タスク
| トラック | 目的 | 主なタスク | 所属ドキュメント | 状態 |
| --- | --- | --- | --- | --- |
| UX/体験 | `/trending` `/following` `/profile/$userId` `/direct-messages` `/search` をブロッカー無しでつなぐ | `phase5_user_flow_inventory.md` 5.1〜5.7 の改善（設定モーダルのプライバシー反映、グローバルコンポーザーからのトピック作成、DM Inboxの仮想スクロール/候補補完、ユーザー検索のレートリミットUI、Summary Panelテレメトリ更新、トレンド/フォローの再実行性担保） | `phase5_user_flow_summary.md`, `tauri_app_implementation_plan.md` Phase3 | ⏳ Stage4（2025年11月12日）: プロフィール Service Worker + Offline ログを `tmp/logs/profile_avatar_sync_stage4_<timestamp>.log` / `nightly.profile-avatar-sync` でクローズし、Topic/Post 導線も Stage4 ログ（`tmp/logs/topic_create_host_20251112-231141.log`, `tmp/logs/topic_create_20251112-231334.log`, `test-results/topic-create/20251112-231334-*.json`, `tmp/logs/post_delete_cache_20251113-085756.log`, `tmp/logs/post-delete-cache_docker_20251113-002140.log`, `test-results/post-delete-cache/20251113-002140.json`）を収集済み。残りは DM Inbox 既読共有の contract テストと `/search` レートリミット UI の仕上げ。 |
| P2P & Discovery | Mainline DHT + Gossip の運用 Runbook を整え、EventGateway 経由でアプリ層へ隠蔽 | 2025年11月13日: EventGateway ポート/mapper を `state.rs` + `LegacyEventManagerGateway` へ注入し (`cargo test --package kukuri-tauri --all-features` ログ: `tmp/logs/cargo-test-kukuri-tauri_di_20251113.log`)、`P2PStack` も `Arc<dyn P2PServiceTrait>` で DI。Runbook Chapter10 の RelayStatus → `apply_cli_bootstrap_nodes` PoC と一致し、`tests/p2p_mainline_smoke.rs` / `tests/integration/topic_create_join.rs` の trait モックで join/broadcast を再検証した。 | `phase5_dependency_inventory_template.md`, `docs/03_implementation/p2p_mainline_runbook.md` | ✅ Gateway/Stack 抽象化完了。次は Offline sync_queue・trending_metrics_job の KPI 仕上げ |
| データ/同期 | Offline ファースト（sync_queue/楽観更新）とトレンド指標自動集計 | `tauri_app_implementation_plan.md` Phase4（sync_queue/offline_actions/競合UI/Service Worker）、`trending_metrics_job` + `scripts/test-docker.{sh,ps1}` `--scenario trending-feed` の自動化、`list_trending_*` の24h集計と `generated_at` ミリ秒保証 | `refactoring_plan_2025-08-08_v3.md` Phase2.5/5.7, `phase5_user_flow_summary.md` 1.2 | ⏳ Stage4（2025年11月11日）: `cache_metadata` Doc/Blob 拡張 + Service Worker + Docker `offline-sync` を `tmp/logs/sync_status_indicator_stage4_<timestamp>.log` / `test-results/offline-sync/*.json` で検証し、`trending_metrics_job` 監視も `tmp/logs/trending_metrics_job_stage4_<timestamp>.log` / `test-results/trending-feed/prometheus/` で固定。次は `sync_engine` の再送メトリクスと `nightly.user-search-pagination` artefact を Runbook/CI へ落とし込む。 |
| Ops/テスト | Nightly/CIでMVP導線を再現しRunbookで復旧できる体制 | `tasks/status/in_progress.md` (GitHub Actions) のトレンドフィードDocker修正、`nightly.yml` `trending-feed` のアーティファクト権限問題切り分け、`docs/01_project/progressReports/` へのRunbookリンク、`scripts/test-docker.ps1 all` の安定化 | `docs/01_project/activeContext/tasks/status/in_progress.md`, `docs/01_project/design_doc.md` | ⏳ 継続調整 |

#### MVP Exit Checklist 連動状況（2025年11月13日）
- **UX/体験導線**: Stage4（プロフィール Service Worker + Offline ログ / Topic & Post Offline シナリオ）を完了。`scripts/test-docker.{sh,ps1} ts --scenario profile-avatar-sync --service-worker` / `./scripts/test-docker.ps1 rust -Test profile_avatar_sync` で `tmp/logs/profile_avatar_sync_stage4_<timestamp>.log` を収集し、`nightly.profile-avatar-sync` artefactへ移行。Topic/Post も `nightly.topic-create`（`tmp/logs/topic_create_host_20251112-231141.log`, `tmp/logs/topic_create_20251112-231334.log`, `test-results/topic-create/20251112-231334-*.json`）と `nightly.post-delete-cache`（`tmp/logs/post_delete_cache_20251113-085756.log`, `tmp/logs/post-delete-cache_docker_20251113-002140.log`, `test-results/post-delete-cache/20251113-002140.json`）で Stage4 をクローズ。DM 既読共有の contract テストと `/search` レートリミット UI が残タスク。→ `phase5_user_flow_summary.md` / `phase5_user_flow_inventory.md`。
- **P2P & Discovery**: 2025年11月13日に EventGateway ポート実装と `P2PStack` trait 化を完了し（`tmp/logs/cargo-test-kukuri-tauri_di_20251113.log`, `tmp/logs/test-docker-rust_di_20251113.log`, `tmp/logs/cargo-test-kukuri-cli_di_20251113.log`）、Runbook Chapter10 / RelayStatus 連携 / CLI 動的更新 PoC（`tmp/logs/relay_status_cli_bootstrap_20251112-094500.log`）を同期済み。次は Offline sync_queue KPI を Mainline Runbook に織り込む。→ `phase5_event_gateway_design.md`, `docs/03_implementation/p2p_mainline_runbook.md`。
- **データ/同期**: `cache_metadata` Doc/Blob 拡張 + Service Worker + conflict banner を 2025年11月11日に実装し、`tmp/logs/sync_status_indicator_stage4_<timestamp>.log` / `./scripts/test-docker.{sh,ps1} ts --scenario offline-sync --no-build` / `test-results/offline-sync/*.json` を Runbook Chapter5 へリンク。`trending_metrics_job` 監視も `prometheus-trending` サービス（`tmp/logs/trending_metrics_job_stage4_<timestamp>.log`, `test-results/trending-feed/prometheus/`）で固定済み。残りは `sync_engine` の再送メトリクスと `nightly.user-search-pagination` artefact の安定化。→ `tauri_app_implementation_plan.md` Phase4, `phase5_ci_path_audit.md`。
- **Ops/CI**: `nightly.topic-create` / `nightly.post-delete-cache` / `nightly.user-search-pagination` を追加し、ログ/JSON artefact を `docs/03_implementation/p2p_mainline_runbook.md` と連動。`corepack enable pnpm` 手順は `phase5_ci_path_audit.md` に固定済みで、残るのは DM contract テストと GitHub Actions 側の `pnpm vitest` キャッシュ戦略。→ `tasks/status/in_progress.md` GitHub Actions 節。

### 2025年11月: Phase 5（MVP仕上げ）
- **Week 1（完了）**: グローバルコンポーザー導線統一、DMモーダルのモックテスト整備、`direct_message_conversations` 永続化。
- **Week 2（完了 2025年11月12日）**:
  - トレンド/フォロー Summary Panel → `trending_metrics_job` 24h 集計と Docker シナリオ固定（`prometheus-trending` 自動起動＋`tmp/logs/trending_metrics_job_stage4_<timestamp>.log`）。
  - プロフィール Stage4: Service Worker / Offline ログ / `cache_metadata` TTL 30 分を Runbook/CI に登録し、`nightly.profile-avatar-sync` で artefact 化。
  - Topic/Post Offline Stage4: `nightly.topic-create` / `nightly.post-delete-cache` を追加し、`tmp/logs/topic_create_*` / `tmp/logs/post_delete_cache_*` / `test-results/*` を収集。
  - `EventGateway` ポート実装 + `P2PStack` trait 化（`tmp/logs/cargo-test-kukuri-tauri_di_20251113.log` 等）。
  - Mainline DHT Runbook / ブートストラップリスト自動更新 PoC（`tmp/logs/relay_status_cli_bootstrap_20251112-094500.log`）を反映。
- **Week 3（進行中）**:
  - DM Inbox 既読共有 + contract テスト、`DirectMessageDialog` 再送ログの Runbook 化。
  - `/search` レートリミット UI + `nightly.user-search-pagination` artefact の固定。
  - GitHub Actions（`pnpm vitest` キャッシュ / trending-feed artefact）と `./scripts/test-docker.ps1 all` の安定化。

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
- **完了**: 認証フロー、リアルタイム同期、トピックCRUD、P2P初期統合、グローバルコンポーザー、トレンド/フォロー Summary Panel + `trending_metrics_job`（2025年11月11日: `prometheus-trending` 監視 & `tmp/logs/trending_metrics_job_stage4_<timestamp>.log` artefact）、Mainline DHT Runbook / ブートストラップ動的更新 PoC（2025年11月12日: `tmp/logs/relay_status_cli_bootstrap_20251112-094500.log`）、プロフィール Stage4（Service Worker + Offline ログ）、Offline sync_queue Stage4（`tmp/logs/sync_status_indicator_stage4_<timestamp>.log`）、Topic/Post Offline Stage4（`nightly.topic-create` / `nightly.post-delete-cache`）、EventGateway + P2PService Stack の trait 化（2025年11月13日）。
- **MVP残**:
  1. DM Inbox 既読共有 + contract テストと `/search` レートリミット UI（`nightly.user-search-pagination`）の最終整備
  2. GitHub Actions（trending-feed artefact / corepack pnpm キャッシュ）と Nightly `scripts/test-docker.ps1 all` フローの安定化
- **Post-MVP**: インセンティブ設計、マーケットプレイスノード、Tauri Mobile

## KPI（2025年11月版）
| 指標 | 目標 | 現状 | 次アクション |
| --- | --- | --- | --- |
| DHT接続成功率 | >95% | 約90%（`p2p_mainline_runbook.md` Chapter10 PoC / `tmp/logs/relay_status_cli_bootstrap_20251112-094500.log`） | EventGateway 実装と `p2p_metrics_export --job p2p` の監視で `apply_cli_bootstrap_nodes` 適用後の再接続率を自動集計する |
| トレンド/フォロー応答時間 | <1.5s | 1.2〜1.6s（`list_trending_*` キャッシュ依存） | `trending_metrics_job` で24h集計 & サマリーパネルのプレフェッチ |
| テストカバレッジ（TS/Rust） | 80% | TS 71% / Rust 68% | `pnpm vitest --coverage` と `cargo tarpaulin` のCI連携 |
| Nightly 成功率 | 100%（連続5日） | 60%（Docker権限問題） | アーティファクトアップロード権限を `ACTIONS_RUNTIME_TOKEN` 不要なモードに切替 |

## 参照ドキュメント
- `docs/01_project/design_doc.md`（MVP Exit Criteria）
- `docs/01_project/refactoring_plan_2025-08-08_v3.md`（技術的負債・Phase5計画）
- `docs/01_project/activeContext/tauri_app_implementation_plan.md`（UI/UXタスク）
- `docs/01_project/activeContext/artefacts/phase5_user_flow_inventory.md`（導線棚卸し）
- `docs/03_implementation/p2p_mainline_runbook.md`（DHT運用方針）
