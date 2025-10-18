# 技術的負債とTODO項目

**最終更新**: 2025年08月16日

## 📋 Phase 7で実装予定のTODO項目

### P2PService関連
1. **message_count実装**
   - ファイル: `application/services/p2p_service.rs`
   - 行: 112
   - 内容: トピックごとのメッセージカウント機能の実装

### OfflineService関連（11件）
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

## 🔧 コード品質改善項目

### TypeScript
- **TODOコメント**: 2件（削減率: 75%）
- **any型使用**: 64箇所
- **未使用APIエンドポイント**: 11件
- **孤立コンポーネント**: 2件
- **ESLint警告**: 17個
  - any型使用に関する警告（テストファイル）
  - Fast Refresh警告（ui/badge.tsx）

### Rust
- **TODOコメント**: 約25件
- **#[allow(dead_code)]**: 97箇所
- **未使用インポート**: 約100件
- **未使用変数**: 約40件
- **デッドコード**: 約29件
- **未実装メソッド**: 約10件

## ⚠️ 注意事項

### Tauriビルド関連
- **Bundle identifier警告**: `com.kukuri.app`が`.app`で終わっている
  - 推奨: `com.kukuri.desktop`などに変更
- **未使用メソッド**: P2Pモジュールの`convert_to_gossip_message`と`extract_topic_ids`
  - 削除または`#[allow(dead_code)]`の追加を検討

### テスト関連
- **act警告**: 一部のReactコンポーネントテストでact警告が発生
  - 主に非同期state更新時に発生
  - 実害はないが、将来的に対応が必要
- **DOM検証警告**: MarkdownPreview.test.tsxで`<div> cannot appear as a descendant of <p>`警告
  - React Markdownコンポーネントの構造に起因
  - 実際の動作には影響なし

### フロントエンド
- **ESLint設定**: src/test/setup.tsで`@typescript-eslint/no-explicit-any`を無効化
  - テストモック実装では型の厳密性よりも柔軟性を優先
- **zustandテスト**: v5対応のモック実装が必要
  - persistミドルウェアも別途モックが必要
  - p2pStoreのテストで特に問題が顕在化

### バックエンド
- **未使用コード**: 多くのモジュールに`#[allow(dead_code)]`が付与
  - 実装時に随時削除する必要がある
- **データベース接続**: 現在は初期化コードのみで、実際の接続処理は未実装
- **P2P統合テスト**: #[ignore]属性でスキップされている

## 📊 統計サマリー

### 実装完了度
- **コマンド移行**: 49/49 (100%)
- **動作確認**: 0/49 (0%)
- **EventService**: 100%
- **P2PService**: 95%（message_countのみ未実装）
- **OfflineService**: 75%（Tauriコマンド／DTO刷新完了、残りは再索引ジョブ周り）

### テストカバレッジ
- **フロントエンド**: 537件実装
- **バックエンド**: 156件実装
- **統合テスト**: 未実装（優先対応が必要）
- **カバレッジ目標**: 最低70%（未設定）

### コード品質
- **TypeScriptエラー**: 0件
- **Rustエラー**: 0件
- **TypeScript警告**: 64件（any型）
- **Rust警告**: 175件（未使用コード）
