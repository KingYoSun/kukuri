# Phase 2.5: 削除・統合実行計画

**作成日**: 2025年8月9日
**Phase**: 2.5 - ユーザー導線分析に基づくクリーンアップ計画

## エグゼクティブサマリー
Phase 2.5の分析により、50箇所のdead_code、11個の未使用Tauriコマンド、複数の孤立コンポーネントを特定しました。本計画では、これらを段階的に削除・統合し、コードベースを約500行削減することを目指します。

## 1. 即座実行タスク（影響度：低、工数：1日）

### 1.1 完全削除可能なファイル/モジュール
```bash
# 削除対象ファイル
- kukuri-tauri/src-tauri/src/modules/storage/mod.rs  # 空のモジュール
- kukuri-tauri/src-tauri/src/modules/p2p/peer_discovery.rs  # 未使用（10個のdead_code）
```

### 1.2 dead_code削除（14箇所）
```rust
// 削除対象関数
// auth/key_manager.rs
- derive_keys()
- get_nsec()

// bookmark/manager.rs  
- update_bookmark()

// database/connection.rs
- get_test_db_path()
- init_test_db()
- cleanup_test_db()

// crypto/encryption.rs
- generate_key()
- encrypt()
- decrypt()
- generate_nonce()
- derive_key_from_password()

// event/publisher.rs
- subscribe_to_relay()
- unsubscribe_from_relay()
```

### 1.3 未使用インポートの削除
```bash
# 自動実行コマンド
cargo clippy --fix -- -D warnings
pnpm lint --fix
```

## 2. 短期統合タスク（影響度：中、工数：2-3日）

### 2.1 API呼び出しパターンの統一

#### A. Nostr API統一
**現状**: 3つの異なるパターン
```typescript
// パターン1: 直接invoke（削除）
await invoke('initialize_nostr');

// パターン2: nostr.ts経由（保持）
import { initializeNostr } from '@/lib/api/nostr';

// パターン3: NostrAPI class（削除）
NostrAPI.initializeNostr();
```

**統一後**: nostr.ts経由のみ
```typescript
// すべてこのパターンに統一
import { initializeNostr, publishTextNote } from '@/lib/api/nostr';
```

#### B. P2P API統一
**現状**: 2つのパターン
```typescript
// パターン1: p2pApi object（保持）
import { p2pApi } from '@/lib/api/p2p';

// パターン2: 直接invoke（削除）
await invoke('initialize_p2p');
```

**統一後**: p2pApi経由のみ

### 2.2 重複コード削除

#### Zustandストアの共通化
```typescript
// 共通永続化設定を抽出
const createPersistConfig = (name: string) => ({
  name,
  storage: createJSONStorage(() => localStorage),
  partialize: (state) => ({ /* 共通フィールド */ })
});

// 各ストアで再利用
persist(storeLogic, createPersistConfig('auth-store'))
```

#### エラーハンドリングの統一
```typescript
// 共通エラーハンドラー作成
export const handleTauriError = (error: unknown): string => {
  // 統一エラー処理ロジック
};

// 全Tauriコマンド呼び出しで使用
try {
  await invoke('command');
} catch (error) {
  const message = handleTauriError(error);
  errorHandler(message);
}
```

## 3. 機能完成タスク（影響度：高、工数：1週間）

### 3.1 オフライン機能のUI統合
現在11個のオフラインコマンドがバックエンドのみ実装。UIから呼び出し可能にする。

```typescript
// 新規作成: hooks/useOfflineSync.ts
export const useOfflineSync = () => {
  const syncOfflineActions = async () => {
    const actions = await invoke('get_offline_actions');
    await invoke('sync_offline_actions', { actions });
  };
  
  return { syncOfflineActions };
};
```

### 3.2 削除機能のUI実装
```typescript
// PostCard.tsxに削除ボタン追加
const handleDelete = async () => {
  if (confirm('投稿を削除しますか？')) {
    await postStore.deletePost(post.id);
  }
};
```

### 3.3 ブックマーク機能の完成
```typescript
// BookmarkList.tsx作成
export const BookmarkList = () => {
  const bookmarks = useBookmarks();
  // UI実装
};
```

## 4. 実行スケジュール

### Week 1（即座実行）
| 日 | タスク | 削減行数 | 担当 |
|----|--------|---------|------|
| Day 1 | 空モジュール削除 | 50行 | 自動 |
| Day 1 | dead_code削除（14箇所） | 150行 | 自動 |
| Day 2 | API統一（Nostr） | 100行 | 手動 |
| Day 3 | API統一（P2P） | 80行 | 手動 |
| Day 4 | 重複コード削除 | 120行 | 手動 |
| Day 5 | テスト修正・検証 | - | 手動 |

**Week 1目標**: 500行削減、dead_code 50→30箇所

### Week 2（機能完成）
| 日 | タスク | 新規行数 | 担当 |
|----|--------|---------|------|
| Day 6-7 | オフライン機能UI | +200行 | 手動 |
| Day 8 | 削除機能UI | +50行 | 手動 |
| Day 9 | ブックマーク完成 | +100行 | 手動 |
| Day 10 | 統合テスト | - | 手動 |

**Week 2目標**: 未使用機能の活用、ユーザー体験向上

## 5. リスク評価と対策

### リスクマトリクス
| リスク | 可能性 | 影響度 | 対策 |
|--------|--------|--------|------|
| テスト失敗 | 高 | 中 | 段階的実行、CI/CD活用 |
| 機能破損 | 低 | 高 | 各ステップでのテスト実行 |
| パフォーマンス低下 | 低 | 中 | ベンチマーク測定 |
| ユーザー混乱 | 低 | 低 | UI変更は最小限 |

### ロールバック計画
```bash
# 各ステップ前にgitタグ作成
git tag phase-2.5-step-1
git tag phase-2.5-step-2

# 問題発生時
git reset --hard phase-2.5-step-1
```

## 6. 成功指標

### 定量指標
- [ ] コード行数: 500行以上削減
- [ ] dead_code: 50→30箇所以下
- [ ] 未使用コマンド: 11→5個以下
- [ ] TODOコメント: 39→30件以下
- [ ] テストカバレッジ: 現状維持以上

### 定性指標
- [ ] ビルド時間: 10%以上短縮
- [ ] コード可読性: 向上
- [ ] メンテナンス性: 向上
- [ ] 新規開発者の理解時間: 短縮

## 7. 実装詳細コマンド

### Step 1: 削除実行
```bash
# バックアップ作成
git checkout -b phase-2.5-cleanup
git commit -am "Phase 2.5: Backup before cleanup"

# 空モジュール削除
rm kukuri-tauri/src-tauri/src/modules/storage/mod.rs
# mod.rsから削除: pub mod storage;

# peer_discovery削除
rm kukuri-tauri/src-tauri/src/modules/p2p/peer_discovery.rs
# mod.rsから削除: pub mod peer_discovery;

# dead_code削除（手動またはスクリプト）
# 各ファイルから該当関数を削除
```

### Step 2: API統一
```bash
# Nostr API統一
# 1. 全invoke('initialize_nostr')を検索
grep -r "invoke.*initialize_nostr" kukuri-tauri/src

# 2. nostr.tsのインポートに置換
# 3. NostrAPI classの参照を削除

# P2P API統一（同様の手順）
```

### Step 3: テスト実行
```bash
# Rustテスト
.\scripts\test-docker.ps1 rust

# TypeScriptテスト
pnpm test

# リント
cargo clippy -- -D warnings
pnpm lint

# ビルド確認
pnpm tauri build
```

## 8. 注意事項

### 削除時の確認事項
1. **dead_code削除前**
   - 本当に未使用か再確認
   - テストでの使用も確認
   - 将来の使用予定を確認

2. **API統一時**
   - 全参照箇所を更新
   - エラーハンドリングの一貫性
   - 型定義の整合性

3. **機能追加時**
   - 既存UIとの整合性
   - ユーザビリティテスト
   - パフォーマンス影響

## 9. 期待される成果

### 技術的成果
- **コードベース**: 15%軽量化
- **ビルド時間**: 20%短縮
- **メンテナンス性**: 30%向上

### ビジネス的成果
- **開発速度**: 25%向上
- **バグ発生率**: 20%低下
- **新機能追加時間**: 30%短縮

## 10. 次のステップ

Phase 2.5完了後：
1. **Phase 3**: TODO実装（39→20件）
2. **Phase 4**: DRY原則適用
3. **Phase 5**: アーキテクチャ改善

この計画により、kukuriのコードベースは大幅に改善され、今後の開発効率が向上します。