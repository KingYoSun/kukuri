[title] 作業中タスク（in_progress）

最終更新日: 2026年01月27日

## 方針（2025年09月15日 更新）

- 当面は Nostr リレーとは接続せず、P2P（iroh + iroh-gossip + DHT）で完結した体験を優先。
- 全イベントは NIPs 準拠のスキーマに沿って取り扱う。
- Tauri v2 では E2E が困難なため、層別テスト（単体・結合/契約＋スモーク最小限）でカバレッジを確保。

## 現在のタスク

### 2025年11月20日 MVP動作確認シナリオ整理

- 目的: Phase 5 Exit Criteria 全項目（`docs/01_project/design_doc.md` / `phase5_user_flow_summary.md`）が実際のアプリ体験として再現できることを確認する。
- 状態: 着手（チェックリスト化のみ完了。次は各項目を `scripts/test-docker.ps1` / Nightly artefactで順次実施）。
- 確認する挙動（想定導線と参照 artefact をすべて列挙）:
  1. **オンボーディング/アカウント/キー管理**
     - `/welcome`→`/login`→`/profile-setup` で `generate_keypair` / `login` / `add_account` / `initialize_nostr` / `update_nostr_metadata` が一連で動作し、複数アカウントを `authStore` から切り替えられる。
     - `ProfileSetup` で入力したプロフィールが Settings > Profile でも同期され、オンボーディング後の再編集時にもドラフト復元が効く。
     - `KeyManagementDialog` から `.nsec` をエクスポート/復旧し、`authStore.loginWithNsec(..., true)` で Secure Storage へ再登録できる（`scripts/test-docker.ps1 ts --filter KeyManagementDialog` / `rust -Test key_management` ログを確認）。
     - ✅ `tests/e2e/specs/onboarding.key-management.spec.ts` でオンボーディング〜キー管理〜複数アカウント切替までを自動検証（msedgedriver 未導入のためローカルWDIO実行は driver 不足で失敗するが、シナリオ自体は自動化済み）
  2. **プロフィール/プライバシー/アバター同期**
     - `ProfileEditDialog` / `SettingsPage` でのプライバシーフラグ更新が `update_privacy_settings`→`authStore.updateUser` に即時反映される。
     - `upload_profile_avatar` → `ProfileAvatarService` → Doc/Blob 保存 → `useProfileAvatarSync.syncNow` の連携でヘッダー画像と `OfflineIndicator` が即更新される。
     - `profileAvatarSyncSW.ts` がバックグラウンド更新・指数バックオフ・`tmp/logs/profile_avatar_sync_*.log` 出力を実施し、`scripts/test-docker.ps1 ts --scenario profile-avatar-sync` / `rust -Test profile_avatar_sync` artefact が取得できる。
  3. **ホーム/トピック/投稿操作**
     - `/` で `PostComposer` のドラフト保存/Markdownプレビュー/添付（必要な場合）が正常動作し、`TopicSelector` のショートカット・最近使用トピックが反映される。
     - `PostCard` からのいいね/ブースト/返信/引用/ブックマーク/ブックマーク解除が `get_posts` / `like_post` / `boost_post` / `bookmark_post` 系 Tauri コマンドと同期して UI が即座に更新される。
     - トピック作成・削除（`nightly.topic-create` / `nightly.post-delete-cache`）が Offline Queue を通じて成功し、`tmp/logs/topic_create_*` / `post_delete_cache_*` / `test-results/topic-*` が Runbook 手順通り採取できる。
  4. **トレンド/フォロー/サマリーパネル**
     - `/trending` で `TrendingSummaryPanel.generated_at` のラグ表示と DM カードが表示され、クリックで対象トピック/DMに遷移する。
     - `/following` でフォロー済みトピックのみが表示され、Follow/Unfollow 操作がサイドバー/TopicSelector/ホームフィードに即時反映される。
     - `trending_metrics_job` → `test-results/trending-feed/{reports,prometheus,metrics}` → UI 表示までのラウンドトリップが `scripts/test-docker.ps1 ts --scenario trending-feed` / Nightly artefactで再現できる。
  5. **Direct Messages（/direct-messages + Profile 経由）**
     - `DirectMessageInbox` の TanStack Virtualizer による会話一覧・検索・未読数が `useDirectMessageBootstrap`（RelayState依存）と同期する。
     - `DirectMessageDialog` で送信/再送/ドラフト保存が Offline Queue を経由して動作し、Kind4 既読共有が複数インスタンス間で同期される（contract: `kukuri-tauri/src-tauri/tests/contract/direct_messages.rs`）。
     - `nightly.direct-message` の `tmp/logs/vitest_direct_message_*.log` / `test-results/direct-message/*.json` が更新され、Runbook Chapter5の DM 手順と突合できる。
  6. **ユーザー検索（/search + 全画面共有コンポーネント）**
     - `useUserSearchQuery` の state machine（allowIncomplete/cooldown/retryAfter）が RateLimit UI と連動して正しく遷移し、補助検索→本検索が自動で切り替わる。
     - 並び替え（関連度/最新順）、無限スクロール、検索エラー時の `SearchErrorState` コンポーネントが `/search` / Sidebar / DM などすべての呼び出し元で共通に機能する。
     - `scripts/test-docker.ps1 ts --scenario user-search-pagination --no-build` の `test-results/user-search-pagination/{reports,logs,search-error}` を Nightly artefact として保存できる。
  7. **SyncStatusIndicator / Offline 同期**
     - `list_sync_queue_items` 60 秒ポーリングと手動再読込が働き、失敗アクションを選択→再試行→成功クリアまで UI で追跡できる。
     - Conflict バナーと `offlineApi.addToSyncQueue` による Doc/Blob（プロフィール・投稿・フォロー・DM）の楽観更新が Stage4 仕様に沿って動作し、`test-results/offline-sync/{topic,post,follow,dm}` と `tmp/logs/sync_status_indicator_stage4_*` が取得できる。
     - `SyncStatusIndicator` が Online/Syncing/Attention Required/Offline の各状態を通知し、Runbook Chapter5 のチェックリストと一致する。
  8. **P2P / RelayStatus / CLI ブートストラップ**
     - RelayStatus カードで `get_relay_status` により Mainline DHT ノード/Peer 数/Bootstrap 情報が表示され、Runbook Chapter10 への導線が機能する。
     - `apply_cli_bootstrap_nodes` で `kukuri-cli --export-path` のリストを UI から適用し、`P2PStack` が `ENABLE_P2P_INTEGRATION=1` で再初期化される。
     - `scripts/test-docker.ps1 integration -NoBuild`（または `./scripts/test-docker.ps1 rust` 内の `p2p_mainline_smoke.rs`）で Mainline DHT 接続ヘルスチェックが再現できる。
  9. **バックグラウンドジョブ / Runbook 連携**
     - `trending_metrics_job` / `nightly.topic-create` / `nightly.post-delete-cache` / `nightly.profile-avatar-sync` / `nightly.sync-status-indicator` の artefact が `.act-artifacts/` と `tmp/logs/*` に揃い、Runbook Chapter4/5/10 の参照先が欠落していない。
     - `docs/03_implementation/p2p_mainline_runbook.md` 記載の採取コマンドで必要ログを収集し、`phase5_ci_path_audit.md` に載っているテスト ID と対応付けられる。
  10. **Ops/CI ガード**
      - `gh act --workflows .github/workflows/test.yml --job format-check` / `--job native-test-linux` が成功し、`.act-artifacts/` に最新ログが保存される。
      - Windows ホストでは必ず `./scripts/test-docker.ps1 ts|rust|all`（必要に応じて `--scenario trending-feed` など）を経由して Vitest / Cargo / ESLint / integration を再実行し、`test-results/` ディレクトリが更新される。
