# リファクタリング計画（改善版）
作成日: 2025年08月08日
最終更新: 2025年11月13日

## 現状分析結果（完全版）

### 1. コードベースの規模
- **フロントエンド (TypeScript/React)**
  - 総行数: 約28,635行
  - ファイル数: 約180ファイル
  - 最大ファイル: PostCard.test.tsx (492行)
  
- **バックエンド (Rust/Tauri)**
  - 総行数: 約9,545行（srcディレクトリのみ）
  - ファイル数: 約50ファイル
  - 最大ファイル: event_sync.rs (554行)

### 2. 技術的負債の詳細

#### 2.1 TODOコメント統計

**TypeScript（8件）:**
1. `Sidebar.tsx:46` - 未読カウント機能の実装
2. `PostComposer.tsx:195` - 画像アップロード機能の実装  
3. `useP2PEventListener.ts:52` - npub変換の実装
4. `useSyncManager.ts:121` - 競合解決UIの表示
5. `useSyncManager.ts:151` - マージロジックの実装
6. `useTopics.ts:20` - メンバー数の取得
7. `useTopics.ts:21` - 投稿数の取得
8. `syncEngine.ts:308` - エンティティメタデータ取得ロジック

**Rust（31件）:**
- `event/handler.rs` - 4件（データベース保存、メタデータ更新、フォロー関係、リアクション処理）
- `p2p/event_sync.rs` - 1件（EventManager統合）
- `p2p/gossip_manager.rs` - 1件（NodeIdパース）
- `p2p/peer_discovery.rs` - 1件（NodeAddrパース）
- `p2p/tests/hybrid_distributor_tests.rs` - 20件（モック実装、テスト実装）
- その他 - 4件

#### 2.2 Dead Codeと警告抑制

**#[allow(dead_code)]の使用状況（97箇所）:**
```
hybrid_distributor.rs    - 24箇所（最多）
event_sync.rs           - 11箇所
peer_discovery.rs       - 10箇所
hybrid_distributor_tests.rs - 10箇所
nostr_client.rs         - 8箇所
gossip_manager.rs       - 6箇所
encryption.rs           - 5箇所
manager.rs (event)      - 4箇所
topic_mesh.rs           - 4箇所
key_manager.rs          - 3箇所
connection.rs           - 3箇所
その他                  - 14箇所
```

**未使用コード:**
- `modules/offline/manager_old.rs` - 古いバージョンのファイル（413行）
- `modules/offline/mod.rs:9` - 未使用のインポート `models::*`

#### 2.3 Clippyエラー（0件／2025年10月31日確認）

2025年10月31日: `kukuri-tauri/src-tauri` と `kukuri-cli` の両ディレクトリで `cargo clippy --workspace --all-features -- -D warnings` を実行し、警告ゼロで完走したことを確認。共通ワークスペースは存在しないため、CI では同コマンドをそれぞれのディレクトリから呼び出す運用に更新する。
2025年11月01日: 指定の 2 ディレクトリで `cargo clippy --all-features -- -D warnings` を再実行し、警告ゼロ継続を確認。実行ログは `docs/01_project/activeContext/artefacts/phase5_ci_path_audit.md` に追記済み。

**再発防止タスク（2025年11月01日追加）:**
- [x] `.github/workflows/test.yml` の lint 系ジョブに kukuri-cli の `cargo clippy --all-features -- -D warnings` を組み込み、Tauri 側と同等の自動チェックを保証する。（2025年11月01日 完了）
- [x] 週次レビューのチェックリストに `phase5_ci_path_audit.md` の lint ログ確認を追加し、記録の欠落を防ぐ運用手順を整備する。（2025年11月01日 完了）

**過去の検出内容（2025年08月時点）:**
1. **未使用インポート（1件）:**
   - `offline/mod.rs:9` - `models::*`の未使用インポート

2. **フォーマット文字列の改善（12件）:**
   - `secure_storage/mod.rs` - 複数箇所でのフォーマット文字列
   - `state.rs:80` - SQLite接続文字列のフォーマット

#### 2.4 テストエラー

**Rust（8件失敗）:**
すべて`modules::offline::tests`モジュール内：
- SQLiteデータベースの書き込み権限エラー
- Docker環境でのパーミッション問題

**TypeScript:**
- エラーなし

#### 2.5 ユーザー導線の現状（Phase 5 棚卸し結果）

- 参照ドキュメント: `docs/01_project/activeContext/artefacts/phase5_user_flow_inventory.md`（2025年11月08日更新）
- 主要導線: サイドバーの `openComposer` 経由で任意画面から投稿モーダルを起動できるほか、`RelayStatus`/`P2PStatus` が 30 秒間隔で接続状況とメトリクスを通知。設定画面の `ProfileEditDialog` は `update_nostr_metadata` → `authStore.updateUser` で即時反映し、ユーザー検索→`/profile/$userId` の導線でフォロー/フォロー解除・フォロワー一覧を閲覧できるようになった。`DirectMessageDialog` は Kind4 IPC ベースの履歴ロードと再送 UI を備え、ヘッダー右上の `MessageCircle` ボタンと `/trending` `/following` の Summary Panel が `useDirectMessageBadge` を共有して未読件数と最新受信時刻をグローバル表示する。
- 残るギャップ: `/trending` `/following` は `trending_metrics_job`（AppState 5分間隔）と Docker `trending-feed`（`tmp/logs/trending_metrics_job_stage4_<timestamp>.log`, `test-results/trending-feed/prometheus/`）まで Stage4 を完了済み。DM 導線も `direct_message_conversations` テーブル・Kind4 IPC・`DirectMessageInbox` の仮想スクロール/候補補完/未読共有を実装し、`tmp/logs/direct_message_inbox_20251113-140827.log`（Vitest）/`tmp/logs/rust_docker_20251113-141846.log`（Docker）で監視している。プロフィール/設定は Stage4（2025年11月12日）で Service Worker + Offline ログ/`cache_metadata` TTL 30 分/`offlineApi.addToSyncQueue` を実装し、Nightly `profile-avatar-sync-logs`（`tmp/logs/profile_avatar_sync_stage4_<timestamp>.log`）まで整備。Topic/Post Offline シナリオも `nightly.topic-create`（`tmp/logs/topic_create_host_20251112-231141.log`, `tmp/logs/topic_create_20251112-231334.log`, `test-results/topic-create/20251112-231334-*.json`）/`nightly.post-delete-cache`（`tmp/logs/post_delete_cache_20251113-085756.log`, `tmp/logs/post-delete-cache_docker_20251113-002140.log`, `test-results/post-delete-cache/20251113-002140.json`）で Stage4 をクローズ。未完リストは、(1) DM Inbox 既読共有の contract テストと `/search` レートリミット UI（`nightly.user-search-pagination`）の仕上げ、(2) `sync_engine` 再送メトリクス/Runbook KPI 化、(3) `join_topic_by_name` / `delete_events` / `add_relay` など未配線 Tauri API 棚卸し、(4) GitHub Actions `docker-test` での `pnpm vitest` キャッシュ・権限調整。

#### 2.6 重複コードパターン

**TypeScript:**
1. **Zustandストア定義** - 8つのストアで同様のpersist設定
2. **テストモック設定** - 複数テストファイルで同じモック実装
3. **エラーハンドリング** - console.error使用箇所

**Rust:**
1. **モック構造体** - `MockEventManager`、`MockGossipManager`の重複
2. **dead_code許可** - 97箇所での同じアノテーション
3. **エラーハンドリング** - println!/eprintln!の多用

## MVPクリティカルギャップ（2025年11月08日更新）

Phase 5 で残っているタスクを MVP 観点で再優先付けした。詳細タスクは以降のフェーズ記述にぶら下げ、進捗とテスト計画を同期させる。

| 領域 | 目的 | 現状 | 次アクション | 参照 |
| --- | --- | --- | --- | --- |
| ユーザーフロー / UX | `/trending` `/following` `/profile/$userId` `/direct-messages` `/search` の導線を「稼働中」に統一 | Summary Panel・DM Inbox・ユーザー検索・設定モーダル（Stage4：プロフィール Service Worker + Offline ログ、Topic/Post Offline Stage4、`trending_metrics_job` 監視）は ✅。`phase5_user_flow_summary.md` で残る未完チェックは DM 既読共有 contract テストと `/search` レートリミット UI（5.4/5.6/5.7/5.10/5.11）。 | `tauri_app_implementation_plan.md` Phase3 の「MVP残タスク」に沿って、`nightly.topic-create` / `nightly.post-delete-cache` / `nightly.profile-avatar-sync` / `nightly.user-search-pagination` artefact を登録済み。引き続き DM/Search backlog をクロージングし、3.3 (リアクション) は Post-MVP として維持。 | `phase5_user_flow_inventory.md`, `phase5_user_flow_summary.md` |
| sync_queue / Offline | 楽観更新＋競合解決で多端末利用時のデータ整合性を確保 | Stage4（2025年11月11日）で `cache_metadata` Doc/Blob 拡張・Service Worker・Docker `offline-sync`（`tmp/logs/sync_status_indicator_stage4_<timestamp>.log`, `test-results/offline-sync/*.json`）を完了し、`nightly.topic-create` / `nightly.post-delete-cache` artefact を追加。`list_sync_queue_items` UI や再送履歴/競合バナー/`OfflineActionType::CREATE_TOPIC` も Runbook Chapter5/`phase5_ci_path_audit.md` に連携済み。 | `tauri_app_implementation_plan.md` Phase4 に従い、残タスクは `sync_engine` 再送メトリクスの可視化と Runbook KPI 化、および `nightly.user-search-pagination` artefact との整合を取るのみ。 | `tauri_app_implementation_plan.md` Phase4 |
| P2P / EventGateway | Application 層から Legacy EventManager を切り離し、Mainline DHT Runbook を整備 | `phase5_event_gateway_design.md` でポート定義済み。2025年11月12日に Runbook Chapter10＋RelayStatus 連携と `kukuri-cli` 動的更新 PoC を完了し、`tmp/logs/relay_status_cli_bootstrap_20251112-094500.log` を Runbook 10.3/10.4 へ記録。 | EventGateway 実装＋`EventService` の依存置換と `P2PService` trait 化に注力。Runbook は保守フェーズに移行したため、High 優先タスクは Gateway 実装／メトリクス監視へリダイレクト。 | `phase5_event_gateway_design.md` |
| テレメトリ/CI | Nightly + Docker でトレンド/フォロー体験を再現し、失敗時にRunbookで復旧 | `trending_feed (Docker)` の`pnpm vitest` 呼び出しとアーティファクト権限調整が継続課題。 | `scripts/test-docker.sh ts --scenario trending-feed` の fixture 分割、`nightly.yml` の権限エラー通知、`docs/01_project/roadmap.md` KPI を更新し、CI 成果物の保存先をS3互換に切替。 | `tasks/status/in_progress.md`, `docs/01_project/roadmap.md` |

## 改善計画

### Phase 0: 緊急対応（1日）

#### 0.1 Clippyエラーの解消
```rust
// 修正例: secure_storage/mod.rs
// Before
println!("SecureStorage: Private key saved successfully for npub={}", npub);
// After
println!("SecureStorage: Private key saved successfully for npub={npub}");
```

#### 0.2 Rustテストエラーの修正
- Docker環境のSQLiteファイルパーミッション修正
- テスト用データベースの適切な初期化

### Phase 1: Dead Code削除（2-3日）

#### 1.1 不要ファイルの削除
- `modules/offline/manager_old.rs` の削除
- 関連する未使用インポートの整理

#### 1.2 #[allow(dead_code)]の精査
優先順位：
1. `hybrid_distributor.rs`（24箇所）
2. `event_sync.rs`（11箇所）
3. `peer_discovery.rs`（10箇所）

**実施手順:**
```bash
# 各dead_code関数の呼び出し元を確認
grep -r "関数名" kukuri-tauri/src --include="*.rs" --include="*.ts"

# 未使用の場合は削除、使用予定の場合はドキュメント化
```

### Phase 2: TODO実装（1週間）

#### 高優先度TODOs
**Rust:**
1. `event/handler.rs` - データベース保存処理（影響度: 高）
2. `p2p/event_sync.rs` - EventManager統合（影響度: 高）

**TypeScript:**
1. `useSyncManager.ts` - 競合解決UI（影響度: 高）
2. `syncEngine.ts` - メタデータ取得ロジック（影響度: 中）

#### 中優先度TODOs
- `useTopics.ts` - カウント機能実装
- `p2p/gossip_manager.rs` - NodeIdパース実装

#### 低優先度TODOs
- テスト関連のTODO（20件）
- UIの細かい改善

### Phase 2.5: ユーザー導線分析と最適化（3日）【新規追加】

#### 2.5.1 未使用機能の特定

**調査対象:**
1. **dead_codeマークされた関数の実使用調査**
   ```bash
   # hybrid_distributor.rs の24箇所を重点調査
   grep -r "distribute\|send\|broadcast" kukuri-tauri/src --include="*.rs" --include="*.ts"
   
   # event_sync.rsの11箇所を調査
   grep -r "sync\|propagate" kukuri-tauri/src --include="*.rs" --include="*.ts"
   ```

2. **Tauriコマンドの使用状況確認**
   ```bash
   # バックエンドで定義されているコマンド一覧
   grep -r "#\[tauri::command\]" kukuri-tauri/src-tauri/src --include="*.rs"
   
   # フロントエンドからの実際の呼び出し
   grep -r "invoke(" kukuri-tauri/src --include="*.ts" --include="*.tsx"
   ```
   - 2025年11月02日: `docs/01_project/activeContext/artefacts/phase5_user_flow_inventory.md` 3.1/3.2/3.3 に呼び出し状況・未導線API・統合テスト専用コマンドを反映し、`TopicPage` の最終更新表示バグ（`lastActive` 秒単位）を改善候補に登録。

3. **ルーティングとページアクセス**
   ```bash
   # 定義されているルート
   find kukuri-tauri/src/routes -name "*.tsx"
   
   # 実際にリンクされているルート
   grep -r "Link to=\|navigate(" kukuri-tauri/src --include="*.tsx"
   ```

#### 2.5.2 機能使用状況の可視化

**ドキュメント作成:**
```markdown
# 機能使用状況マップ
## アクティブな機能
- [ ] 機能名: 呼び出し元 → 実装箇所
  
## 未使用の機能（削除候補）
- [ ] 機能名: 実装箇所（dead_code）
  
## 部分的に使用されている機能
- [ ] 機能名: 使用箇所 / 未使用箇所
```

**作成状況（2025年11月14日更新）**
- [x] アクティブな機能: `docs/01_project/activeContext/artefacts/phase5_feature_usage_map.md` に UI 導線／フロント実装／Tauri コマンドを一覧化。
- [ ] 未使用の機能（削除候補）: 未接続 API と dead_code の棚卸しを追記する。
- [ ] 部分的に使用されている機能: 一部導線のみで使われる UI/コマンドの使用/未使用文脈を書き分ける。

#### 2.5.3 削除・統合計画

**削除対象:**
1. UIから到達不可能なコマンド
2. 呼び出し元が存在しないdead_code
3. テスト専用コードの本番混入

**統合対象:**
1. 重複している機能
2. 類似の処理を行う複数の関数

### Phase 3: ファイル分割（1週間）

**注意: 700行を超えるファイルは現在存在しないため、将来の予防的措置として実施**

#### TypeScriptファイル分割（予防的）
- 現在最大のPostCard.test.tsx（492行）が700行に近づいた場合に分割

#### Rustファイル分割（予防的）
- event_sync.rs（554行）が700行に近づいた場合に分割計画を実行

### Phase 4: DRY原則適用（1週間）

#### 4.1 共通化実装

**Rust共通モック:**
```rust
// tests/common/mock.rs
pub trait MockManager {
    fn new() -> Self;
    fn with_config(config: Config) -> Self;
    fn setup_expectations(&mut self);
}

// 使用例
impl MockManager for MockEventManager {
    fn new() -> Self { /* 実装 */ }
    fn with_config(config: Config) -> Self { /* 実装 */ }
    fn setup_expectations(&mut self) { /* 実装 */ }
}
```

**TypeScript共通ストア:**
```typescript
// stores/utils/createPersistedStore.ts
export function createPersistedStore<T>(
  name: string,
  initialState: T,
  actions: (set: SetState<T>, get: GetState<T>) => Partial<T>
) {
  return create<T>()(
    persist(
      (set, get) => ({
        ...initialState,
        ...actions(set, get),
      }),
      {
        name,
        storage: createJSONStorage(() => localStorage),
      }
    )
  );
}
```

#### 4.2 エラーハンドリング統一

**Rust:**
```rust
// logging設定
use log::{info, warn, error};

// Before: println!("SecureStorage: Private key saved");
// After:  info!("SecureStorage: Private key saved");
```

**TypeScript:**
```typescript
// すべてのconsole.errorをerrorHandlerに置換
import { errorHandler } from '@/lib/errorHandler';

// Before: console.error('Error:', error);
// After:  errorHandler.handle(error);
```

### Phase 5: アーキテクチャ改善（2週間）

#### 5.1 Rustモジュール再構成
```
src/
├── domain/         // ビジネスロジック
│   ├── entities/
│   └── usecases/
├── infrastructure/ // 外部連携
│   ├── tauri/
│   ├── p2p/
│   └── storage/
├── application/    // アプリケーション層
│   └── services/
└── presentation/   // コマンド層
    └── commands/
```

#### 5.2 テスト構造の改善
```
tests/
├── unit/          // ユニットテスト
├── integration/   // 統合テスト
├── common/        // 共通ユーティリティ
│   ├── mocks/
│   └── fixtures/
└── e2e/           // E2Eテスト
```

#### 5.3 実装計画

- **目的と前提**  
  - Phase 1〜4 で整理済みの TODO/DRY 対応が完了している状態を前提とし、`docs/01_project/refactoring_plan_2025-08-08_v3.md` に定義したレイヤ構成への移行を完了させる。  
  - 既存モジュール間の依存関係を把握するため、着手前に `cargo tree --edges features` と `cargo modules generate graph`（未導入の場合はテンポラリで導入）で依存図を生成し、主要サービス／Repository／コマンドの依存先を一覧化する。

- **準備ステップ**  
  1. 依存関係棚卸し: 主要クレート構成とモジュール依存を表形式で整理し、レイヤ移行可否（切り離し難易度、循環依存の有無）を評価する。  
  2. テスト棚卸し: 既存テスト（`modules/*/tests` など）をユニット／統合／共通ユーティリティに分類し、移動対象のファイルと不足領域をリスト化する。  
  3. CI 影響調査: 現行の GitHub Actions／ローカルスクリプト（`scripts/test-*.sh`, `./scripts/test-docker.ps1`）で参照しているパスを確認し、構成変更時の修正ポイントをまとめる。

- **Workstream A: Rustモジュール再構成（Week4 前半〜中盤）**  
  1. `domain`,`application`,`infrastructure`,`presentation` 配下に段階移行用の `mod.rs` とプレースホルダーを作成し、既存パスとの互換を確保するための一時的な `pub use` を定義する。  
  2. ドメインロジック（エンティティ／ユースケース）と外部連携実装（DB・P2P・Tauri）を切り出し、下位レイヤから上位レイヤへの依存のみ許容するルールを `cargo deny` もしくは簡易スクリプトで検証する。  
  3. DI 初期化（`ApplicationContainer` 等）と Tauri コマンドバインディングを新レイヤに合わせて再配線し、プレゼンテーション層→アプリケーション層→ドメイン層の一方向依存を担保する。  
  4. 互換フェーズの間は旧モジュールパスに対する `pub use` を残し、段階的に呼び出し元を更新してから削除する。  
  5. 各移行ステップ後に `cargo fmt` / `cargo clippy -D warnings` / `cargo test` を実行し、リグレッションを即時検知する。
  - **棚卸し結果（2025年10月24日更新）**  
    | Legacyモジュール | 現状/主依存 | 移行ターゲット | 段階移行案 | 参照 |
    | --- | --- | --- | --- | --- |
    | `infrastructure::event::{manager,handler,nostr_client_manager}` | EventManager が Legacy KeyManager・旧ハンドラー構成に依存。DB 接続は `ConnectionPool` へ統一済み（2025年10月25日）。 | `application::ports::EventGateway` + `infrastructure::event::EventManagerGateway` | Stage1（完了 2025年10月24日）: Gateway 導入済み。<br>Stage2（完了 2025年10月27日）: EventManager/Handler/NostrClient を Infrastructure 層へ移設し、DI から `Arc<dyn EventGateway>` を注入。<br>Stage3（完了 2025年10月25日）: `EventManagerHandle` で `modules::event` 参照を封じ、`tests/integration/test_event_service_gateway.rs` で送信パスの結合テストを追加。 | `phase5_event_gateway_design.md`<br>`phase5_dependency_inventory_template.md` |
    | `modules::offline::{manager,reindex}` | OfflineManager/OfflineReindexJob が SQLx クエリと JSON 変換を直接保持し、Application 層へ Legacy 型をリーク。 | `application::ports::OfflinePersistence` + `infrastructure::offline::*` | Stage0（完了 2025年10月24日）: ドメイン値オブジェクト追加。<br>Stage1（Week4 0.7 目標）: `LegacyOfflineManagerAdapter` を介してポートに接続。<br>Stage2（Week4 1.0 目標）: `infrastructure/offline` 実装を導入し SQLx ロジックを移植。<br>Stage3（Week5 0.3 目標）: Legacy モジュール縮退と `.sqlx` 更新。 | `phase5_offline_adapter_plan.md` |
    | `modules::bookmark` | **完了 2025年10月26日**: BookmarkRepository + PostService への移行を完了し、`modules/bookmark`（manager/tests/types）を削除。 | `domain::entities::bookmark` + `infrastructure::database::BookmarkRepository` + `application::services::PostService` 拡張 | Stage0〜2: 2025年10月25日までに実装済み。<br>Stage3（2025年10月26日）: Legacy Manager/テストをアーカイブし、ドキュメント・タスク・Runbook を更新。 | `phase5_dependency_inventory_template.md` 更新済み<br>`tauri_app_implementation_plan.md` |
  | `modules::secure_storage` | **完了 2025年10月25日**: Legacy SecureStorage ユーティリティを `infrastructure::storage::secure_storage::DefaultSecureStorage::clear_all_accounts_for_test` へ移管し、モジュール/テストを削除済み。 | `infrastructure::storage::secure_storage::DefaultSecureStorage` | Stage1（完了 2025年10月25日）: Debug/テスト用ユーティリティを移植し、`clear_all_accounts_for_test` コマンドを新実装へ接続。<br>Stage2（完了 2025年10月25日）: Legacy モジュールとテストを削除し、TypeScript/Tauri 依存を刷新。<br>Stage3（完了 2025年10月25日）: 依存棚卸しと Runbook を更新し、debug 手順を最新状態に揃えた。 | `phase5_dependency_inventory_template.md`（更新） |
  | `modules::auth::KeyManager` | `AppState` や Legacy EventManager が同期 API を直接呼び出し、`nostr_sdk::Keys` を保持。 | `application::ports::key_manager` + `infrastructure::crypto::DefaultKeyManager` | Stage1（完了 2025年10月25日）: `application::ports::key_manager` を新設し、`Arc<dyn KeyManager>` ベースで `AppState` / SecureStorage Handler / Tauri コマンドを再配線。<br>Stage2（完了 2025年10月25日）: `LegacyEventManagerGateway` / `EventManagerHandle` / `SubscriptionInvoker` を新ポート経由に移し、Gateway/Topic/Post 系テストを更新。<br>Stage3（完了 2025年10月25日）: Legacy モジュールと再エクスポートを削除し、依存ドキュメントを更新。 | `phase5_dependency_inventory_template.md` |
  | `modules::crypto::encryption` | AES-256-GCM を直接ラップしたユーティリティで、`AppState` の暗号機能と SecureStorage テストが依存。 | `infrastructure::crypto::encryption_service`（新設予定） + `shared::security` | Stage0（Week4 0.4 目標）: 暗号化トレイト定義とテストダブル作成。<br>Stage1（Week4 0.7 目標）: `AppState`/SecureStorage テストを新トレイトに切替。<br>Stage2（Week4 0.9 目標）: Legacy モジュール削除。 | `phase5_dependency_inventory_template.md`（新規行） |
  | `modules::database::{connection,models}` | **完了 2025年10月25日**: `ConnectionPool` へ全面移行済み。ファイル削除。 | `infrastructure::database::{ConnectionPool,SqliteRepository}` | Stage1〜3 完了: `state`／`EventManager`／`EventHandler` を再配線し、Docker Rust テストで回帰確認。 | `phase5_dependency_inventory_template.md`<br>`phase5_offline_adapter_plan.md` |
  - **進捗（2025年10月31日更新）**  
    - `application::services::p2p_service` を `core` / `builder` / `status` 構成と `tests` ディレクトリへ分割し、モックの再利用と責務境界を明確化。  
    - `domain::entities::event` を `model` / `kind` / `validation` 階層へ再構成し、NIP/BOLT 検証ロジックをサブモジュール化。テストは `tests/` 配下で NIP-01・NIP-10/19・kind30078 ごとに整理した。
  - **実装順序（2025年10月24日確定）**  
    1. **WSA-01 EventGateway Stage 2**  
       - `phase5_event_gateway_design.md` Sprint 2 を実行し、`LegacyEventManagerGateway` を `infrastructure::event` へ移設。  
       - `state/application_container.rs` と `presentation::handlers` を `Arc<dyn EventGateway>` 経由に再配線。  
       - 前提: Sprint 1 で trait / mapper が導入済み（完了）。  
    2. **WSA-02 Offline Persistence Stage 1/2**  
       - `application::ports::offline_store` を実装し、`LegacyOfflineManagerAdapter` で既存 Manager を包む（Stage1）。  
       - `infrastructure/offline/sqlite_store.rs` を新設し、Stage2 で Sqlite 実装へ差し替え。  
       - 参考: `phase5_offline_adapter_plan.md`（Stage1〜2）。WSA-01 完了後に着手。  
    3. **WSA-03 Bookmark Repository 移行**  
       - `domain::entities::bookmark` / `infrastructure::database::bookmark_repository` を追加し、`PostService` に新 Repository を注入。  
       - `presentation::handlers::post_handler` の bookmark 系 API を新サービス経由に揃え、Legacy BookmarkManager を縮退。  
       - 参考: `phase5_dependency_inventory_template.md` BookmarkManager 行。WSA-02 完了後に着手。  
       - 2025年10月26日: Stage3（Legacy BookmarkManager アーカイブ）を完了し、`modules::bookmark` ディレクトリとユニットテストを削除。タスク／依存棚卸し／Runbook を更新し、Bookmark API は PostService + BookmarkRepository に一本化された。  
    4. **WSA-04 SecureStorage / Encryption 再配線**  
       - （完了 2025年10月25日）`infrastructure::storage::secure_storage::DefaultSecureStorage::clear_all_accounts_for_test` を追加し、debug/テストユーティリティと `SecureStorageHandler` の依存を刷新。  
       - `infrastructure::crypto` に暗号化トレイトを追加し、`AppState` が Legacy EncryptionManager に頼らないよう再構成。  
       - 参考: `phase5_dependency_inventory_template.md` Legacy SecureStorage / EncryptionManager 行。WSA-03 完了後に着手。  
    5. **WSA-05 Legacy Database Connection 廃止**  
       - `modules::database::connection` を段階的に削除し、全呼び出しを `ConnectionPool` + Repository 経由に揃える。（完了 2025年10月25日）  
       - Offline/Bookmark の Repository 移行が完了していることを前提とし、`.sqlx` 再生成とタスク整理を実施。  
       - 参考: `phase5_dependency_inventory_template.md` Legacy Database Connection 行。WSA-04 完了後に着手。  

- **Workstream B: テスト構造再編（Week4 後半〜Week5 前半）**  
  1. `tests/unit`,`tests/integration`,`tests/common/{mocks,fixtures}` を作成し、既存テストを種類に応じて再配置する。  
  2. 共通モック・フィクスチャを `tests/common` に集約し、重複定義を削減するユーティリティモジュールを整備する。  
  3. `Cargo.toml` の `[dev-dependencies]` / `[[test]]` 設定を更新して新ディレクトリを参照させ、`cargo test -- --test-threads=1` など既存カスタム呼び出しの動作確認を行う。  
  4. CI とローカルスクリプトでユニット／統合テストを個別に実行できるようジョブを分割し、`docs/03_implementation/p2p_mainline_runbook.md` に手順を追記する。  
  5. 統合テストのスモークケースを追加し、`cargo tarpaulin` 等でカバレッジを採取して成功指標の更新方法を確立する。

- **マイルストーン**  
  - Week4 0.5: 依存棚卸しとテスト棚卸し完了、移行マップ共有。  
  - Week4 1.0: Workstream A の主要移行（DI 再配線まで）完了、互換パスでビルド成功。  
  - Week5 0.5: Workstream B 完了、CI とローカルスクリプト更新済み。  
  - Week5 1.0: `cargo test` / `pnpm test` がグリーンで成功指標のチェックリスト更新、`docs/01_project/progressReports/` へレポート登録。

- **ドキュメント／タスク連携**  
  - `docs/01_project/activeContext/tauri_app_implementation_plan.md` にレイヤ再構成の進捗を反映し、影響範囲と未完了項目を記録する。  
  - 運用観点の更新は `docs/03_implementation/p2p_mainline_runbook.md` に記載し、タスクステータスは `tasks/status/in_progress.md` で管理する。

- **リスクと緩和策**  
  - 循環依存の発生: 依存棚卸し時に循環候補を洗い出し、ドメインサービスの責務分割で対応する。  
  - テスト移動によるパス破損: 移動後即座に対象テストのみ `cargo test --test <name>` で実行し、失敗時は差分最小化を優先する。  
  - CI 設定漏れ: 変更前後でワークフローを比較し、レビュー用チェックリスト（CI手順・スクリプト）を活用する。

- **即時アクション（Day0）**  
  1. 依存関係棚卸し用のテンプレート表（CSV/スプレッドシート想定）の項目定義を決定する。  
  2. `cargo tree --edges features > docs/01_project/activeContext/artefacts/cargo_tree_phase5.txt` など、基礎データを取得して共有可能な artefact を作成する（`artefacts` ディレクトリが無い場合は作成する）。  
  3. `tasks/status/in_progress.md` に Phase 5 実装計画タスクを追記し、計画実行フェーズの進捗管理を開始する。

## 成功指標（改善版）

### 技術的指標
- [x] Clippyエラー0件（2025年11月01日再確認済み）
- [x] TODOコメント50%削減（39件→20件以下）
- [x] #[allow(dead_code)]を50%削減（97件→50件以下）
- [x] 700行超のファイル0件（現在0件を維持） — 2025年11月01日: `infrastructure/p2p/event_distributor` を責務別モジュールへ分割し、`application/services/p2p_service` を `core` / `bootstrap` / `metrics` 構成に再編。
- [x] manager_old.rsの削除
- [ ] すべてのRustテスト成功
- [ ] コード重複率30%削減

### ユーザー導線指標【新規追加】
- [x] UIから到達可能な全機能の文書化完了 — `docs/01_project/activeContext/artefacts/phase5_user_flow_inventory.md` に導線マップとギャップ分析を追記（2025年11月01日作成、2025年11月02日: グローバル導線と統合テスト専用コマンドを追記）
  - 2025年11月03日: 同ドキュメント 1.2/1.6 にサイドバーのグローバルコンポーザー連携と `RelayStatus`/`P2PStatus` の監視内容、設定画面のプロフィール編集モーダル詳細を追加。
  - 2025年11月04日: 同ドキュメント 1.4/1.7/5.6 にユーザー検索の実装状況、`/profile/$userId` 導線、フォロー体験の優先課題を追記。
  - 2025年11月06日: 同ドキュメント 5.7-5.10 と `phase5_user_flow_summary.md` の 1.2/2/3 にトレンド/フォロー導線、DM 未読バッジ、投稿削除後のキャッシュ整合性、Docker シナリオを追記し、CI 監査・実装計画とのリンクを同期。
  - 2025年11月06日: `useOfflineStore` / `useSyncManager` から `update_cache_metadata` と `update_sync_status` を呼び出す実装を追加し、バックエンドの同期メタデータと SyncStatusIndicator の表示が連動することを確認。`offlineStore.test.ts` にメタデータ更新ケースを追加。
  - 2025年11月07日: Inventory 5.6.1/5.6.2 と Summary 2章に `/profile/$userId` の DM 起点導線・フォロー/フォロワー一覧のソート/検索/件数表示、`DirectMessageDialog` の Kind4 IPC・未読バッジ・再送ボタン実装状況を反映。`profile.$userId.test.tsx` を Nightly に追加し、Rust（`kukuri-cli`）と TypeScript のテスト結果を記録。
  - 2025年11月07日: Inventory 5.11 と Summary Quick View に `SyncStatusIndicator` / `OfflineIndicator` の役割分担、`useSyncManager` + `offlineStore` + `offlineApi.update_cache_metadata/update_sync_status` の流れ、`get_cache_status` / `add_to_sync_queue` 連携、Vitest (`useSyncManager.test.tsx`, `SyncStatusIndicator.test.tsx`) を追記し、同期導線の不足を可視化。
  - 2025年11月09日: Summary に「MVP Exit Checklist（2025年11月09日版）」、Inventory に Sec.0「MVP Exit クロスウォーク」を追加。4カテゴリ（UX/体験 / P2P & Discovery / データ・同期 / Ops・CI）の担当セクションとテストログ（`tmp/logs/*.log`）の参照先を明示し、`tauri_app_implementation_plan.md` のステータス欄と連携させた。
- [ ] 未使用APIエンドポイント0件
  - 2025年11月02日: 同ドキュメント 3.2/3.3 に未導線API（`delete_post` / `add_relay` など）とテスト専用 `invoke` コマンドを整理。UI導線追加または削除方針の判断待ち。
  - 2025年11月06日: Kind4 DM 系コマンド（`send_direct_message` / `list_direct_messages`）を UI 導線へ接続し、Inventory 3.2 と 5.6 の未使用一覧を更新。残る候補は `add_relay` / `join_topic_by_name` などに絞り、削除方針は Phase 5 backlog で継続検討。
  - 2025年11月07日: `get_cache_status` / `add_to_sync_queue` を `useSyncManager`・`SyncStatusIndicator` へ組み込み、Inventory 5.11 の未接続一覧から除外。未導線 API は `add_relay` / `join_topic_by_name` / `delete_events` / `get_nostr_pubkey` など最小グループに整理し、Phase 5 backlog と同期。
  - 2025年11月09日: `phase5_dependency_inventory_template.md` の MVP注視モジュール表を更新し、EventGateway / P2PService Stack / OfflineService / TopicSelector ショートカットの優先度と `phase5_user_flow_summary.md` クロスウォークをリンク。`add_relay` / `join_topic_by_name` は Phase 5 backlog のまま、`profile_avatar_sync` Doc/Blob 対応と紐付けて整理。
- [ ] 孤立コンポーネント0件
  - 2025年11月06日: `TrendingSummaryPanel` / `FollowingSummaryPanel` / `DirectMessageDialog` の導線を Inventory 5.7・Summary 2 に反映し、Sidebar・Header からの呼び出し経路とテストケースを整理。未接続要素（鍵管理ダイアログ等）は Inventory 5.4 の backlog として明示。
  - 2025年11月07日: `SyncStatusIndicator`・`OfflineIndicator` の UI 役割と `SyncStatusIndicator` → `useSyncManager` → `offlineStore` のバッジ/手動再送導線を Inventory 5.11 / Summary Quick View / Phase5 優先度リストへ反映し、孤立していた同期ステータス UI を主要導線に統合した。
- [ ] dead_codeのうち80%以上が削除または使用開始
  - 2025年11月06日: Inventory 5.7-5.10 で各導線に紐づくストア/API 呼び出しを洗い出し、未参照だった削除/集計系のコードを対象外として明文化。dead_code 候補は `hybrid_distributor` / `event_sync` 等バックエンド側に集約した状態を Phase 5 backlog に記録。
  - 2025年11月07日: Inventory 5.11 で `update_cache_metadata` / `update_sync_status` / `get_cache_status` / `add_to_sync_queue` の利用パスをテキスト化し、`offline_api.rs` の dead_code 候補（未呼び出しだった同期系コマンド）を全て使用中へ更新。`phase5_ci_path_audit.md` に新テスト ID を登録し、除外候補リストから同期 API 群を削除。
- [ ] すべてのTauriコマンドがフロントエンドから呼び出し可能
  - 2025年11月06日: Inventory 3.2/3.3 に DM/トレンド関連コマンドの呼び出し箇所を追記し、`phase5_user_flow_summary.md` で導線ステータスを更新。未接続コマンドは Phase 5 backlog へ移管し、CI パス監査との整合を確認。
  - 2025年11月07日: `get_cache_status` / `add_to_sync_queue` / `update_cache_metadata` / `update_sync_status` の呼び出し経路を Inventory 5.11 に追記し、`phase5_user_flow_summary.md` でもグローバル要素「同期導線」を更新。`useSyncManager.test.tsx` / `SyncStatusIndicator.test.tsx` を追加し、CI ドキュメントとリンクした。

## リスク管理

### 高リスク項目
1. **dead_code削除による機能破壊**
   - 対策: 削除前に全文検索で参照確認
   - ロールバック計画: Gitでの段階的コミット

2. **TODO実装による新規バグ**
   - 対策: テスト駆動開発
   - カバレッジ目標: 80%以上

3. **ユーザー導線分析での誤削除**
   - 対策: 削除前に2段階確認プロセス
   - 1段階目: コード検索での使用確認
   - 2段階目: 実行時の動作確認

## 実行スケジュール

| 週 | Phase | 内容 | 成果物 |
|---|-------|------|--------|
| 1日目 | Phase 0 | Clippyエラー修正、テスト修正 | エラー0件 |
| 週1前半 | Phase 1 | Dead Code削除 | dead_code 50%削減 |
| 週1後半 | Phase 2.5 | ユーザー導線分析 | 機能使用マップ作成 |
| 週2 | Phase 2 | 高優先度TODO実装 | 重要機能の完成 |
| 週3 | Phase 4 | DRY原則適用 | 重複コード削減 |
| 週4-5 | Phase 5 | アーキテクチャ改善 | 保守性向上 |

## 次のアクション

### 即座に実行（Day 1）
1. **Clippyエラーの修正と検証**
   ```bash
   cd kukuri-tauri/src-tauri
   cargo clippy --fix --workspace --all-features
   cargo clippy --workspace --all-features -- -D warnings
   cd ../../kukuri-cli
   cargo clippy --workspace --all-features -- -D warnings
   ```
   - 共通ワークスペースは存在しないため、Tauri アプリと CLI の両方で `cargo clippy --workspace --all-features -- -D warnings` を実行し、警告ゼロを確認する。

2. **manager_old.rsの削除確認**
   ```bash
   grep -r "manager_old" kukuri-tauri/src-tauri/src
   # 参照がなければ削除
   ```

### 今週中に実行（Week 1）
1. **ユーザー導線の調査開始**
   - 全Tauriコマンドの使用状況調査
   - dead_code関数の呼び出し元確認

2. **dead_code精査開始**
   - hybrid_distributor.rsから開始
   - 各関数の必要性を判断

### 継続的に実行
1. **進捗の週次レビュー**
   - 成功指標の達成状況確認
   - 計画の調整
   - `docs/01_project/activeContext/artefacts/phase5_ci_path_audit.md` の lint ログ更新状況を確認し、最新記録の有無とギャップをレビュー

2. **ドキュメント更新**
   - 機能使用マップの維持
   - 削除・変更の記録

## 付録: 調査コマンド集

```bash
# dead_code関数の使用状況確認
find kukuri-tauri/src-tauri/src -name "*.rs" -exec grep -l "#\[allow(dead_code)\]" {} \; | \
  xargs -I {} sh -c 'echo "=== {} ===" && grep -A 1 "#\[allow(dead_code)\]" {}'

# Tauriコマンドの対応確認
echo "=== Backend Commands ===" && \
  grep -h "#\[tauri::command\]" -A 1 kukuri-tauri/src-tauri/src/**/*.rs | \
  grep "pub async fn\|pub fn" | sed 's/.*fn \([^(]*\).*/\1/' | sort

echo "=== Frontend Invocations ===" && \
  grep -h "invoke(" kukuri-tauri/src/**/*.ts kukuri-tauri/src/**/*.tsx | \
  sed "s/.*invoke[(<]\s*['\"]\\([^'\"]*\\).*/\\1/" | sort | uniq

# 未使用インポートの検出
cargo clippy --workspace --all-features -- -W unused-imports
```

---

このリファクタリング計画は、コード品質の向上と技術的負債の削減に加え、
**ユーザー導線の最適化**を重視して作成されました。
特にdead_codeの97箇所とTODOの39件は、実使用状況を確認した上で
適切に対処する必要があります。
