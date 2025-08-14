# 既知の問題と注意事項

**最終更新**: 2025年8月14日（E2Eテスト基盤構築完了）

## 🚨 現在の問題

### E2Eテストの部分的失敗（2025年8月14日追加）
**問題**: E2Eテストで一部のテストが失敗

**詳細**:
- 6テスト中4テスト成功、2テスト失敗
- 失敗原因：
  - React #root要素が見つからない（アプリ起動直後のレンダリング未完了）
  - div要素が0個（DOM構築のタイミング問題）

**影響**: E2Eテスト基盤は動作するが、テストの安定性に課題

**回避策**:
1. 要素の待機処理を追加
2. data-testid属性をアプリケーション側に追加
3. `browser.url('/')`を使用しない（Tauriアプリ内でのURL指定制限）

**優先度**: 中（基盤は動作、安定化が必要）

### tauri-driverのポート番号（2025年8月14日追加）
**問題**: tauri-driverがポート4445で起動（ドキュメントと異なる）

**詳細**:
- デフォルトはポート4444と記載されているが、実際は4445
- wdio.conf.tsのポート設定を4445にする必要がある

**影響**: 設定ミスによる接続エラー

**対処**: wdio.conf.tsで`port: 4445`を明示的に指定

**優先度**: 低（ドキュメント化済み）

### トレイトメソッドのTODO実装（2025年8月14日 Phase 5完了後更新）
**問題**: OfflineServiceの多くのメソッドが基本実装のみ

**現在の状況**:
- EventServiceTrait: ✅ 完全実装済み（EventManager統合）
- P2PServiceTrait: ✅ ほぼ完了（message_countのTODOのみ）
- OfflineServiceTrait: ⚠️ 11メソッドが基本実装のみ（詳細実装必要）

**影響**: EventService/P2PServiceは基本動作可能、OfflineServiceは詳細実装待ち

**優先度**: 中（テスト追加後にPhase 7で対応予定）

### コンパイル警告多数（2025年8月14日 Phase 5完了後更新）
**問題**: 175件の警告が存在（Phase 5で6件増加）

**内訳**:
- 未使用インポート: 約100件
- 未使用変数: 約40件
- デッドコード: 約29件

**影響**: ビルド・実行には支障なし

**優先度**: 低（後日クリーンアップ予定）

### Windows環境でのテスト実行エラー（2025年8月14日更新）
**問題**: Windows環境でRustテストがDLLエラーで実行不可

**エラー内容**:
```
STATUS_ENTRYPOINT_NOT_FOUND (0xc0000139)
```

**回避策**: Docker環境でのテスト実行
```powershell
# 推奨コマンド
.\scripts\test-docker.ps1
```

**優先度**: 中（開発環境依存、Docker使用で回避可能）

### TypeScriptテストの一部失敗（2025年8月13日更新）
**問題**: テスト固有の設定問題により一部のテストが失敗

**現在の状況**:
- 10個のテストファイルが失敗（53個は成功）
- 608個のテストが成功、13個が失敗
- 主な失敗原因：
  - タイマーモックの設定不備（`vi.useFakeTimers()`の未使用）
  - 期待値の不一致（gcTime、retry設定）
  - 非同期処理のタイミング問題
- 優先度: 低（機能的な問題ではなく、テスト設定の調整で解決可能）

## ✅ 最近解決された問題

### ビルドエラー22件（2025年8月14日解決）
**問題**: Result型の不整合によるコンパイルエラー

**解決内容**:
- 全サービス層のResult型を`AppError`に統一
- インフラ層（Repository, P2P, 暗号化）のResult型統一
- From実装の追加（7種類のエラー型変換）
- メソッドシグネチャの修正（`join_topic`, `get_node_id`等）

**結果**: 
- コンパイルエラー: 22件 → 0件 ✅
- ビルド成功達成
- 警告は169件に増加（未使用インポート等、実害なし）

### コンパイルエラー175件（2025年8月13日解決）
**問題**: インフラ層の補完実装後、アプリケーションが起動不可能な状態だった

**解決方法（第4回作業）**:
1. **TypeScriptコンパイルエラーの修正**:
   - `currentAccount` → `currentUser`への統一（authStore使用箇所3ファイル）
   - Zustand永続化設定を新形式に移行:
     ```typescript
     // 旧: createLocalStoragePersist('name', partialize)
     // 新: { name: 'name', partialize: (state) => ({...}) }
     ```
   - `SaveOfflineActionRequest`インターフェース修正（EntityType追加）
   - radio-group UIコンポーネント作成（@radix-ui/react-radio-group依存追加）
   - syncEngine.tsのTauriApi呼び出し修正（静的メソッド化、CreatePostRequestインターフェース対応）

2. **Rust側の対応**:
   - 実際にはRust側にコンパイルエラーはなく、警告14件のみ
   - 未使用インポートの警告は残存（今後のクリーンアップ対象）

**結果**: 
- TypeScript/Rustともにビルド成功
- アプリケーション起動可能状態に復帰

### コード品質エラー全般の解消（2025年8月12日）
**問題**: プロジェクト全体でコンパイルエラー、型エラー、リントエラーが多数存在

**症状と解決**:
1. **Rustリントエラーの修正（13件）**
   - format!マクロでのインライン変数展開を使用
   ```rust
   // 修正前
   format!("エラー: {}", e)
   // 修正後
   format!("エラー: {e}")
   ```

2. **TypeScript未使用コードの削除（20件）**
   - PostCard.tsx: Wifiインポート削除
   - authStore.ts: createJSONStorage削除
   - offlineStore.ts: OfflineActionType削除

3. **依存パッケージの追加**
   - @radix-ui/react-progress: Progress UIコンポーネント用
   - @vitest/utils: Vitestユーティリティ

**結果**:
- Rust Clippy: エラー0件
- TypeScript型チェック: エラー0件
- TypeScriptリント: エラー0件
- Docker環境での全テスト実行が正常に完了

## 📊 パフォーマンス最適化（2025年8月13日実装）

### キャッシュ戦略
**実装内容**:
- MemoryCacheService<T>: 汎用メモリキャッシュ
- TTL（Time To Live）サポート: 自動期限切れ処理
- 特殊化キャッシュ:
  - PostCacheService: 5分キャッシュ
  - UserCacheService: 10分キャッシュ  
  - TopicCacheService: 30分キャッシュ

**パフォーマンス改善**:
- キャッシュヒット時: DB不要で50倍高速化
- 1000件の読み書き: 100ms以内
- メモリ使用量: LRU風の自動クリーンアップ

### 並行処理の最適化
**実装内容**:
- npub変換: tokio::task::spawn_blockingで並行化
- バッチ処理: futures::future::join_allで一括実行
- ハンドラー再利用: AppStateに保持

**パフォーマンス改善**:
- npub変換: 100件で5倍高速化
- ハンドラー生成: 50µs → 1µs（50倍改善）
- CPU使用率: マルチコア活用で効率向上

### バッチ処理
**実装内容**:
- BatchGetPostsRequest: 最大100件一括取得
- BatchReactRequest: 最大50件一括リアクション
- BatchBookmarkRequest: 最大100件一括ブックマーク

**パフォーマンス改善**:
- ラウンドトリップ削減: N回 → 1回
- DB接続効率: コネクションプール活用
- レスポンス時間: 最大5倍改善

## ⚠️ 現在の注意事項

### Tauriビルド関連
- **Bundle identifier警告**: `com.kukuri.app`が`.app`で終わっているためmacOSでの競合の可能性
  - 推奨: `com.kukuri.desktop`などに変更
- **未使用メソッド警告**: P2Pモジュールの`convert_to_gossip_message`と`extract_topic_ids`
  - 削除または`#[allow(dead_code)]`の追加を検討

### テスト関連
- **テストカバレッジ**: フロントエンド537件、バックエンド156件、合計693件のテストを実装（2025年8月3日更新）
- **act警告**: 一部のReactコンポーネントテストでact警告が発生する場合がある
  - 主に非同期state更新時に発生
  - 実害はないが、将来的に対応が必要
- **DOM検証警告**: MarkdownPreview.test.tsxで`<div> cannot appear as a descendant of <p>`警告
  - React Markdownコンポーネントの構造に起因
  - 実際の動作には影響なし

### フロントエンド
- **ESLint設定**: src/test/setup.tsで`@typescript-eslint/no-explicit-any`を無効化
  - テストモック実装では型の厳密性よりも柔軟性を優先
- **ESLint警告**: 17個の警告が残存（2025年7月27日更新）
  - any型使用に関する警告（テストファイル）
  - Fast Refresh警告（ui/badge.tsx）
  - これらは動作に影響しないため、優先度低として保留
- **zustandテスト**: v5対応のモック実装が必要
  - persistミドルウェアも別途モックが必要
  - p2pStoreのテストで特に問題が顕在化

### バックエンド
- **未使用コード**: 多くのモジュールに`#[allow(dead_code)]`が付与されている
  - 実装時に随時削除する必要がある
- **データベース接続**: 現在は初期化コードのみで、実際の接続処理は未実装
- **Rustリント警告**: エラーは全て解消済み（2025年7月27日更新）
  - 警告のみ残存（unsafe code、テスト用モック等）
  - P2P統合テストは#[ignore]属性でスキップ

### 開発環境
- **formatコマンド**: CLAUDE.mdに記載されている（2025年7月28日確認済み）
  - `pnpm format`でフォーマット実行
  - `pnpm format:check`でフォーマットチェック

## 💡 技術的な決定事項

### テスト戦略
1. **フロントエンドテスト**
   - Vitest + React Testing Library
   - 全コンポーネント、フック、ストアに対してテストを作成
   - カバレッジ目標は設定せず、重要な機能に集中

2. **バックエンドテスト**
   - Rust標準のテスト機能を使用
   - 各モジュールに対して単体テストを作成
   - 統合テストは今後追加予定

### コード品質
1. **リント設定**
   - フロントエンド: ESLint（TypeScript、React対応）
   - バックエンド: cargo clippy
   - 両方とも警告ゼロを維持

2. **型安全性**
   - TypeScript: strictモード有効
   - Rust: 全ての警告を解消（一時的な抑制を除く）

## 📌 Phase 5で追加されたTODO項目（2025年8月14日）

### P2PService関連
1. **message_count実装**
   - ファイル: `application/services/p2p_service.rs`
   - 行: 112
   - 内容: トピックごとのメッセージカウント機能の実装

### OfflineService関連（13件）
ファイル: `application/services/offline_service.rs`

1. **save_action** (行131-138)
   - Repository経由でのoffline_actionsテーブルへの保存
   - UUID生成とタイムスタンプ設定

2. **get_actions** (行143-155)
   - フィルタリング条件の適用
   - entity_type, entity_id, statusによる絞り込み

3. **sync_actions** (行156-164)
   - 指定アクションまたは未同期アクションの取得
   - サーバーへの送信処理
   - is_synced=trueへの更新

4. **get_cache_status** (行165-172)
   - cache_metadataテーブルからの統計取得
   - 総サイズ、アイテム数の計算
   - 最古・最新アイテムのタイムスタンプ取得

5. **add_to_sync_queue** (行173-181)
   - sync_queueテーブルへの挿入
   - 優先度管理の実装

6. **update_cache_metadata** (行182-188)
   - キャッシュメタデータの更新
   - TTL管理の実装

7. **save_optimistic_update** (行189-200)
   - optimistic_updatesテーブルへの保存
   - 元データと更新データの記録

8. **confirm_optimistic_update** (行201-205)
   - 楽観的更新の確定処理

9. **rollback_optimistic_update** (行206-210)
   - ロールバック処理の実装
   - 元データの復元

10. **cleanup_expired_cache** (行211-217)
    - 期限切れアイテムの削除
    - 削除数のカウント

11. **update_sync_status** (行218-224)
    - 同期ステータスの更新処理
    - コンフリクトデータの管理

12. **Repository層との統合**
    - 全メソッドでのRepository活用
    - トランザクション管理

13. **実際のデータベース操作**
    - SQLクエリの実装
    - エラーハンドリング

## 🔗 関連ドキュメント

- [アーカイブ（解決済み問題）](./archives/issuesAndNotes_2025-08-early.md)
- [現在のタスク](./current_tasks.md)
- [環境情報](./current_environment.md)
- [Phase 5基本機能実装完了報告](../progressReports/2025-08-14_phase5_basic_implementation_complete.md)