# v2アーキテクチャへのコマンド移行 - Phase 1

**作成日**: 2025年08月14日  
**作業者**: Claude  
**フェーズ**: 新アーキテクチャ移行作業

## 概要

新アーキテクチャへの完全移行に向けて、modules/*ディレクトリ内の旧コマンドをv2アーキテクチャに移行する作業の第1段階を実施しました。認証関連とセキュアストレージ関連の基本コマンド9個の移行を完了し、ビルドエラーを完全に解消しました。

## 作業内容

### 1. 移行状況の調査

modules/*ディレクトリの全コマンドを調査し、以下の未移行コマンドを特定：

- **認証関連**: 3コマンド
- **セキュアストレージ**: 6コマンド
- **Nostrイベント**: 10コマンド
- **P2P関連**: 7コマンド
- **オフライン関連**: 11コマンド
- **ユーティリティ**: 2コマンド
- **合計**: 約40コマンド

### 2. 認証関連コマンドのv2移行

#### 実装ファイル
- `presentation/commands/auth_commands_v2.rs` (新規作成)
- `presentation/handlers/auth_handler.rs` (既存活用)

#### 移行完了コマンド
1. `generate_keypair` → `generate_keypair_v2`
2. `login` → `login_v2`
3. `logout` → `logout_v2`

#### 実装のポイント
- AuthHandlerを経由した統一的な実装
- 旧インターフェースとの互換性維持
- エラーハンドリングの統一（AppError使用）

### 3. セキュアストレージコマンドのv2移行

#### 実装ファイル
- `presentation/commands/secure_storage_commands_v2.rs` (新規作成)
- `presentation/handlers/secure_storage_handler.rs` (新規作成)

#### 移行完了コマンド
1. `add_account` → `add_account_v2`
2. `list_accounts` → `list_accounts_v2`
3. `switch_account` → `switch_account_v2`
4. `remove_account` → `remove_account_v2`
5. `get_current_account` → `get_current_account_v2`
6. `secure_login` → `secure_login_v2`

#### 実装のポイント
- SecureStorageHandlerの新規実装
- DefaultSecureStorageの静的メソッド呼び出しに対応
- AccountMetadataの適切な管理
- AuthServiceとの連携による認証処理

### 4. ビルドエラーの解消

#### 修正内容
- 重複定義の解消（旧auth_commands.rsの重複関数をコメントアウト）
- AuthHandlerのエクスポート追加
- AppError::DatabaseError → AppError::Databaseへの修正
- SecureStorageHandlerの構造修正（静的メソッド対応）
- インポートパスの整理

#### エラー解消の成果
- **コンパイルエラー**: 175件 → 0件 ✅
- **ビルド**: 成功 ✅

## 技術的詳細

### アーキテクチャパターン
```
[Tauriコマンド] → [v2コマンド] → [Handler] → [Service] → [Infrastructure]
```

### ハンドラー実装例
```rust
pub struct SecureStorageHandler {
    auth_service: Arc<AuthService>,
}

impl SecureStorageHandler {
    pub fn new(auth_service: Arc<AuthService>) -> Self {
        Self { auth_service }
    }
    
    pub async fn add_account(&self, request: AddAccountRequest) -> Result<AddAccountResponse, AppError> {
        // 入力検証
        request.validate()?;
        
        // サービス層での処理
        let user = self.auth_service.login_with_nsec(&request.nsec).await?;
        
        // インフラ層での永続化
        DefaultSecureStorage::add_account(/* ... */)?;
        
        Ok(response)
    }
}
```

## 統計

### 移行進捗
- **移行完了**: 9コマンド / 約40コマンド（22.5%）
- **新規作成ファイル**: 3ファイル
- **修正ファイル**: 5ファイル

### コード量
- `auth_commands_v2.rs`: 78行
- `secure_storage_commands_v2.rs`: 95行
- `secure_storage_handler.rs`: 161行
- **合計**: 334行の新規コード

### ビルド状況
- **コンパイルエラー**: 0件
- **警告**: 177件（主に未使用インポート）
- **ビルド時間**: 51.93秒（リリースビルド）

## 課題と今後の対応

### 残作業
1. **Nostrイベントコマンド（10個）**
   - publish_text_note、send_reaction等の移行が必要
   
2. **P2P関連コマンド（7個）**
   - join_p2p_topic、broadcast_to_topic等の移行が必要
   
3. **オフライン関連コマンド（11個）**
   - save_offline_action、sync_offline_actions等の移行が必要
   
4. **ユーティリティコマンド（2個）**
   - pubkey_to_npub、npub_to_pubkey の移行が必要

### 技術的課題
- 警告177件の削減（未使用インポート、未使用関数）
- modules/*ディレクトリの最終的な削除
- テストカバレッジの確保

## 次のステップ

1. Nostrイベントコマンド10個のv2移行
2. P2P関連コマンド7個のv2移行
3. オフライン関連コマンド11個のv2移行
4. ユーティリティコマンド2個のv2移行
5. 警告の大幅削減
6. modules/*ディレクトリの削除
7. 包括的なテスト実行

## まとめ

Phase 1では認証とセキュアストレージという基本的かつ重要な機能のv2移行を完了しました。ビルドエラーを完全に解消し、アプリケーションがビルド可能な状態を維持しながら、段階的な移行を進めています。残り約30コマンドの移行により、新アーキテクチャへの完全移行が完了する見込みです。