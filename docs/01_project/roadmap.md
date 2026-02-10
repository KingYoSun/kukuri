# kukuri プロジェクトロードマップ

**作成日**: 2025年08月16日  
**最終更新**: 2026年02月10日

## プロジェクトビジョン
kukuri は Nostr イベントをベースにしたトピック中心ソーシャルクライアントとして、BitTorrent Mainline DHT + iroh を土台に完全分散なP2P体験を提供する。MVP のゴールは、トレンド/フォロー/DM/プロフィール/検索といった日常の導線を Mainline DHT 上で安定提供し、Nightly + Runbook で再現できる状態まで引き上げることにある。

## ロードマップ概要（2025年11月20日再編）
- Phase 1〜2（認証/トピック/リアルタイム）は完了済み。Phase 3.3〜5 も 2025年11月20日に Exit Criteria を満たし、MVP を達成した。
- Discovery は Mainline DHT + `kukuri-community-node`（`cn-cli`）ブートストラップのみを運用対象とする方針へ移行中（2026年02月10日更新）。
- トラックは **UX/体験**, **P2P & Discovery**, **データ同期**, **Ops/テスト** の4分類で管理し、成果物は `docs/01_project/design_doc.md`・`phase5_user_flow_summary.md` と同期している。
- 今後は 12月前半を Phase 7（リリース準備・Runbook最終化）、12月末をベータリリースに充てる。

### MVPトラック別の残タスク
| トラック | 目的 | 主なタスク | 所属ドキュメント | 状態 |
| --- | --- | --- | --- | --- |
| UX/体験 | `/trending` `/following` `/profile/$userId` `/direct-messages` `/search` をブロッカー無しでつなぐ | `phase5_user_flow_inventory.md` 5.1〜5.7（プライバシー設定導線、グローバルコンポーザー、DM Inbox 仮想スクロールと既読共有、検索レートリミット UI、Summary Panel テレメトリ、トレンド/フォロー再実行性） | `phase5_user_flow_summary.md`, `tauri_app_implementation_plan.md` Phase3 | ✅ 2025年11月20日: `test-results/{trending-feed,direct-message,user-search-pagination}` と `tmp/logs/vitest_direct_message_*.log` で導線カバレッジを固定。DM 既読 contract テスト・`/search` RateLimit UI も Nightly artefact に反映済み。 |
| P2P & Discovery | Mainline DHT + Gossip の運用 Runbook を整え、EventGateway 経由でアプリ層へ隠蔽 | EventGateway ポート/mapper, `P2PStack` DI, Runbook Chapter10, `cn p2p bootstrap --export-path` + `apply_cli_bootstrap_nodes` PoC | `phase5_dependency_inventory_template.md`, `docs/03_implementation/p2p_mainline_runbook.md` | ✅ Gateway/Stack 抽象化後、`p2p_mainline_smoke.rs`・`scripts/test-docker.ps1 integration` と Runbook を同期。`p2p_metrics_export` でノード状態を定期採取し、DHT 経路のみを運用対象としている。 |
| データ/同期 | Offline ファースト（sync_queue/楽観更新）とトレンド指標自動集計 | `tauri_app_implementation_plan.md` Phase4、`trending_metrics_job`、`offline_metrics.rs`、Service Worker/BroadcastChannel、自動再送・競合 UI | `docs/01_project/deprecated/refactoring_plan_2025-08-08_v3.md`（アーカイブ） Phase2.5/5.7, `phase5_user_flow_summary.md` 1.2 | ✅ `trending_metrics_job` / `offline-sync` / `profile-avatar-sync` artefact（`test-results/trending-feed/{reports,prometheus,metrics}`、`test-results/offline-sync/{topic,post,follow,dm}`）と `tmp/logs/sync_status_indicator_stage4_*` を Nightly へ固定。`SyncStatusIndicator` で `OfflineRetryMetrics` を監視。 |
| Ops/テスト | Nightly/CIでMVP導線を再現しRunbookで復旧できる体制 | `nightly.*` シナリオ整備、`github/test.yml` (`native-test-linux`,`format-check`)、`scripts/test-docker.ps1 all`、`.act-artifacts/` でのログ転送 | `docs/01_project/activeContext/tasks/status/in_progress.md`, `docs/01_project/design_doc.md` | ✅ `nightly.topic-create/post-delete/profile-avatar-sync/trending-feed/user-search-pagination/sync-status-indicator` と GH Actions ジョブを整備し、`gh act --workflows .github/workflows/test.yml` のログを `.act-artifacts/` に保存。 |

#### MVP Exit Checklist 連動状況（2025年11月20日）
- **UX/体験導線**: Stage4 完了後に DM 既読 contract テスト（`kukuri-tauri/src-tauri/tests/contract/direct_messages.rs`）と `/search` RateLimit UI を反映。`nightly.trending-feed` / `nightly.direct-message` / `nightly.user-search-pagination` artefact で導線を継続検証し、`phase5_user_flow_summary.md` の「未実装」は 0 件。
- **P2P & Discovery**: EventGateway 抽象化と `P2PStack` trait 化を反映し、Runbook Chapter10 + CLI PoC + `p2p_metrics_export` で Mainline DHT のヘルスチェックを自動化。Nightly `integration` ジョブで `ENABLE_P2P_INTEGRATION=1` を再現。
- **データ/同期**: `trending_metrics_job`・SyncStatusIndicator・Service Worker・`offline_metrics.rs` を Nightly に組み込み、`test-results/trending-feed/{reports,prometheus,metrics}` や `test-results/offline-sync/{category}` を Runbook Chapter5 から参照可能にした。
- **Ops/CI**: `nightly.topic-create/post-delete/profile-avatar-sync/trending-feed/user-search-pagination/sync-status-indicator` と `github/test.yml` (`native-test-linux`,`format-check`) を `.act-artifacts/` でトレース。`phase5_ci_path_audit.md` に GitHub Actions のキャッシュ/権限課題と回避策を記録した。

#### 2025年11月下旬のフォローアップ（作業中）
- `/welcome` → `/profile-setup` で止まる E2E 導線の再検証と `authStore.generateNewKeypair/loginWithNsec` 周辺ログの拡充を進行中（2025年11月21日時点）。プロフィールセットアップ後のリダイレクト抑止を確認するため、E2E ブリッジ環境での再実行が必要。
- MVP 動作確認シナリオ（Phase5 Exit Checklist）の手動/スモーク再現を `scripts/test-docker.ps1 all|ts|rust` と Nightly artefact (`test-results/**`, `tmp/logs/**`) で継続検証中。チェックリスト化は完了済みで、次は `scripts/test-docker.ps1` 実行ログの最新化を予定。

### 2025年11月: Phase 5（MVP仕上げ）
- **Week 1（完了）**: グローバルコンポーザー導線の統一、DMモーダルのモックテスト整備、`direct_message_conversations` 永続化。
- **Week 2（完了 2025年11月13日）**:
  - トレンド/フォロー Summary Panel → `trending_metrics_job` 24h 集計と Docker シナリオ固定（`prometheus-trending` 自動起動＋`tmp/logs/trending_metrics_job_stage4_<timestamp>.log`）。
  - プロフィール Stage4（Service Worker / Offline ログ / `cache_metadata` TTL 30 分）を `nightly.profile-avatar-sync` artefact に統合。
  - Topic/Post Offline Stage4: `nightly.topic-create` / `nightly.post-delete-cache` を追加し、`tmp/logs/topic_create_*` / `tmp/logs/post_delete_cache_*` / `test-results/*` を収集。
  - EventGateway ポート実装 + `P2PStack` trait 化（`tmp/logs/cargo-test-kukuri-tauri_di_20251113.log` 等）。
- **Week 3（完了 2025年11月20日）**:
  - DM Inbox 既読共有 + contract テスト、`DirectMessageDialog` 再送ログの Runbook 化。
  - `/search` レートリミット UI + `nightly.user-search-pagination` artefact（SearchErrorState JSON 含む）を固定。
  - GitHub Actions（`pnpm vitest` キャッシュ / trending-feed artefact）と `./scripts/test-docker.ps1 all` を整備し、MVP Exit 判定を締結。
