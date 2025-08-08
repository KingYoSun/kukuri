# リファクタリング計画（改善版）
作成日: 2025年8月8日
最終更新: 2025年8月8日

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