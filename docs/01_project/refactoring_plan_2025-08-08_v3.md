# リファクタリング計画（改善版）
作成日: 2025年08月08日
最終更新: 2025年08月08日

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

#### 2.3 Clippyエラー（13件）

**主なエラータイプ:**
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

#### 2.5 ユーザー導線の現状（未調査）

**調査が必要な項目:**
- UIから到達不可能なAPIエンドポイント
- dead_codeマークされた関数の実際の使用状況
- 孤立したコンポーネントやページ
- 実装されているが使用されていない機能

#### 2.6 重複コードパターン

**TypeScript:**
1. **Zustandストア定義** - 8つのストアで同様のpersist設定
2. **テストモック設定** - 複数テストファイルで同じモック実装
3. **エラーハンドリング** - console.error使用箇所

**Rust:**
1. **モック構造体** - `MockEventManager`、`MockGossipManager`の重複
2. **dead_code許可** - 97箇所での同じアノテーション
3. **エラーハンドリング** - println!/eprintln!の多用

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
    | `modules::event::{manager,handler,nostr_client}` | EventManager が Legacy KeyManager・旧ハンドラー構成に依存。DB 接続は `ConnectionPool` へ統一済み（2025年10月25日）。 | `application::ports::EventGateway` + `infrastructure::event::EventManagerGateway` | Stage1（完了 2025年10月24日）: Gateway 導入済み。<br>Stage2（継続）: LegacyEventManagerGateway を Infrastructure 層へ移設し、DI から `Arc<dyn EventGateway>` を注入。<br>Stage3（完了 2025年10月25日）: `EventManagerHandle` で `modules::event` 参照を封じ、`tests/integration/test_event_service_gateway.rs` で送信パスの結合テストを追加。 | `phase5_event_gateway_design.md`<br>`phase5_dependency_inventory_template.md` |
    | `modules::offline::{manager,reindex}` | OfflineManager/OfflineReindexJob が SQLx クエリと JSON 変換を直接保持し、Application 層へ Legacy 型をリーク。 | `application::ports::OfflinePersistence` + `infrastructure::offline::*` | Stage0（完了 2025年10月24日）: ドメイン値オブジェクト追加。<br>Stage1（Week4 0.7 目標）: `LegacyOfflineManagerAdapter` を介してポートに接続。<br>Stage2（Week4 1.0 目標）: `infrastructure/offline` 実装を導入し SQLx ロジックを移植。<br>Stage3（Week5 0.3 目標）: Legacy モジュール縮退と `.sqlx` 更新。 | `phase5_offline_adapter_plan.md` |
    | `modules::bookmark` | BookmarkManager が `SqlitePool` を直接扱い `AppState` からのみ参照。`PostService` の bookmark 系 API は未実装。 | `domain::entities::bookmark` + `infrastructure::database::BookmarkRepository` + `application::services::PostService` 拡張 | Stage0（Week4 0.3 目標）: Bookmark 値オブジェクトと Repository トレイトを設計。<br>Stage1（Week4 0.6 目標）: `infrastructure::database::bookmark_repository` を追加し、`PostService` へ注入。<br>Stage2（Week4 0.8 目標）: Tauri コマンド／UI を新サービス経由に切替。<br>Stage3（Week5 0.2 目標）: Legacy Manager・テストを削除。 | `phase5_dependency_inventory_template.md` 更新予定<br>`tauri_app_implementation_plan.md` |
    | `modules::secure_storage` | **完了 2025年10月25日**: Legacy SecureStorage ユーティリティを `infrastructure::storage::secure_storage::DefaultSecureStorage::clear_all_accounts_for_test` へ移管し、モジュール/テストを削除済み。 | `infrastructure::storage::secure_storage::DefaultSecureStorage` | Stage1（完了 2025年10月25日）: Debug/テスト用ユーティリティを移植し、`clear_all_accounts_for_test` コマンドを新実装へ接続。<br>Stage2（完了 2025年10月25日）: Legacy モジュールとテストを削除し、TypeScript/Tauri 依存を刷新。<br>Stage3（完了 2025年10月25日）: 依存棚卸しと Runbook を更新し、debug 手順を最新状態に揃えた。 | `phase5_dependency_inventory_template.md`（更新） |
    | `modules::auth::KeyManager` | `AppState` や Legacy EventManager が同期 API を直接呼び出し、`nostr_sdk::Keys` を保持。 | `application::ports::key_manager` + `infrastructure::crypto::DefaultKeyManager` | Stage1（完了 2025年10月25日）: `application::ports::key_manager` を新設し、`Arc<dyn KeyManager>` ベースで `AppState` / SecureStorage Handler / Tauri コマンドを再配線。<br>Stage2（完了 2025年10月25日）: `LegacyEventManagerGateway` / `EventManagerHandle` / `SubscriptionInvoker` を新ポート経由に移し、Gateway/Topic/Post 系テストを更新。<br>Stage3（完了 2025年10月25日）: Legacy モジュールと再エクスポートを削除し、依存ドキュメントを更新。 | `phase5_dependency_inventory_template.md` |
    | `modules::crypto::encryption` | AES-256-GCM を直接ラップしたユーティリティで、`AppState` の暗号機能と SecureStorage テストが依存。 | `infrastructure::crypto::encryption_service`（新設予定） + `shared::security` | Stage0（Week4 0.4 目標）: 暗号化トレイト定義とテストダブル作成。<br>Stage1（Week4 0.7 目標）: `AppState`/SecureStorage テストを新トレイトに切替。<br>Stage2（Week4 0.9 目標）: Legacy モジュール削除。 | `phase5_dependency_inventory_template.md`（新規行） |
    | `modules::database::{connection,models}` | **完了 2025年10月25日**: `ConnectionPool` へ全面移行済み。ファイル削除。 | `infrastructure::database::{ConnectionPool,SqliteRepository}` | Stage1〜3 完了: `state`／`EventManager`／`EventHandler` を再配線し、Docker Rust テストで回帰確認。 | `phase5_dependency_inventory_template.md`<br>`phase5_offline_adapter_plan.md` |
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
- [ ] Clippyエラー0件
- [ ] TODOコメント50%削減（39件→20件以下）
- [ ] #[allow(dead_code)]を50%削減（97件→50件以下）
- [ ] 700行超のファイル0件（現在0件を維持）
- [ ] manager_old.rsの削除
- [ ] すべてのRustテスト成功
- [ ] コード重複率30%削減

### ユーザー導線指標【新規追加】
- [ ] UIから到達可能な全機能の文書化完了
- [ ] 未使用APIエンドポイント0件
- [ ] 孤立コンポーネント0件
- [ ] dead_codeのうち80%以上が削除または使用開始
- [ ] すべてのTauriコマンドがフロントエンドから呼び出し可能

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
1. **Clippyエラーの修正**
   ```bash
   cd kukuri-tauri/src-tauri
   cargo clippy --fix --workspace --all-features
   ```

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
