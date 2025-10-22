# 進捗レポート: セキュアストレージと複数アカウント管理機能の実装

**日付**: 2025年07月28日  
**作成者**: AI Assistant  
**カテゴリ**: セキュリティ、認証、UX改善

## 概要

Tauriアプリケーションのセキュリティとユーザー体験を大幅に改善するため、プラットフォーム固有のセキュアストレージを活用した複数アカウント管理機能を実装しました。これにより、ユーザーは安全に複数のNostrアカウントを管理し、素早く切り替えることができるようになりました。

## 実装背景

### 問題点
1. **セキュリティリスク**: 秘密鍵（nsec）がlocalStorageに保存されていた
2. **UX課題**: アプリを開くたびにログインが必要
3. **アカウント管理**: 複数アカウントの切り替えが困難

### 解決方針
- プラットフォーム固有のセキュアストレージ（Keychain等）の活用
- 自動ログイン機能の実装
- 複数アカウントの一元管理

## 実装内容

### 1. Rustバックエンド実装

#### セキュアストレージモジュール (`secure_storage/mod.rs`)
```rust
// keyring crateを使用してプラットフォーム固有のストレージにアクセス
pub struct SecureStorage;

impl SecureStorage {
    // 秘密鍵を安全に保存（npubごとに個別管理）
    pub fn save_private_key(npub: &str, nsec: &str) -> Result<()>
    
    // アカウントメタデータの管理（公開情報のみ）
    pub fn save_accounts_metadata(metadata: &AccountsMetadata) -> Result<()>
    
    // 複数アカウントの切り替え
    pub fn switch_account(npub: &str) -> Result<()>
}
```

#### プラットフォーム対応
- **macOS**: Keychain Services
- **Windows**: Credential Manager
- **Linux**: Secret Service (GNOME Keyring等)

### 2. Tauriコマンド実装

新しく追加されたコマンド：
- `add_account` - アカウントを追加してセキュアに保存
- `list_accounts` - 保存済みアカウント一覧を取得
- `switch_account` - アカウントを切り替え
- `remove_account` - アカウントを削除
- `get_current_account` - 現在のアカウントを取得（自動ログイン用）
- `secure_login` - セキュアストレージからログイン

### 3. フロントエンド実装

#### SecureStorageApi (`lib/api/secureStorage.ts`)
```typescript
export const SecureStorageApi = {
  async addAccount(request: AddAccountRequest): Promise<AddAccountResponse>,
  async listAccounts(): Promise<AccountMetadata[]>,
  async switchAccount(npub: string): Promise<SwitchAccountResponse>,
  async removeAccount(npub: string): Promise<void>,
  async getCurrentAccount(): Promise<GetCurrentAccountResponse | null>,
  async secureLogin(npub: string): Promise<LoginResponse>,
};
```

#### authStoreの拡張
```typescript
interface AuthStore extends AuthState {
  accounts: AccountMetadata[];
  loginWithNsec: (nsec: string, saveToSecureStorage?: boolean) => Promise<void>;
  generateNewKeypair: (saveToSecureStorage?: boolean) => Promise<{ nsec: string }>;
  switchAccount: (npub: string) => Promise<void>;
  removeAccount: (npub: string) => Promise<void>;
  loadAccounts: () => Promise<void>;
}
```

#### 自動ログイン機能
```typescript
initialize: async () => {
  // セキュアストレージから現在のアカウントを取得
  const currentAccount = await SecureStorageApi.getCurrentAccount();
  
  if (currentAccount) {
    // 自動ログイン実行
    // Nostrクライアントを初期化
    // リレー状態を更新
  }
}
```

### 4. UIコンポーネント

#### AccountSwitcher コンポーネント
- ヘッダーに統合されたドロップダウンメニュー
- 現在のアカウント表示
- アカウント切り替え機能
- アカウント削除機能
- 新規アカウント追加へのリンク

#### LoginForm の改良
- 「アカウントを安全に保存」チェックボックス追加
- デフォルトでセキュアストレージに保存

## セキュリティ改善

### 1. 秘密鍵の保護
- LocalStorageには一切保存されない
- プラットフォームのセキュアストレージで暗号化
- メモリに保持せず、必要時のみ取得

### 2. アクセス制御
- アプリケーションごとにアクセス制限
- OSレベルの認証（Touch ID、Windows Hello等）と連携可能

### 3. 複数アカウント管理
- 各アカウントの秘密鍵は個別に暗号化保存
- アカウントメタデータは公開情報のみ保存
- 最終使用日時でソート表示

## テスト実装

### 1. Rustバックエンドテスト
- モックストレージを使用した単体テスト（8件）
- 秘密鍵の保存・取得・削除
- アカウントメタデータ管理
- 複数アカウント切り替え

### 2. フロントエンドテスト
- SecureStorageApiテスト（6テストスイート）
- authStore統合テスト（5テストスイート）
- 複数アカウント統合テスト（3テストスイート）

### 3. テストカバレッジ
- API層: 100%
- ストア層: 主要機能をカバー
- 統合テスト: 実際のワークフローを検証

## ユーザー体験の向上

### 1. 自動ログイン
- アプリ起動時に最後に使用したアカウントで自動ログイン
- 「リダイレクト中...」の表示を解消

### 2. アカウント切り替え
- ワンクリックでアカウント切り替え
- 切り替え時にNostrクライアントを自動再初期化

### 3. アカウント管理
- 複数のNostrアカウントを一元管理
- アカウントごとにプロフィール画像とメタデータを表示
- 不要なアカウントを簡単に削除

## 技術的な考慮事項

### 1. エラーハンドリング
- セキュアストレージアクセス失敗時のフォールバック
- アカウントが見つからない場合の適切な処理

### 2. 互換性
- keyring crateがサポートする全プラットフォームで動作
- セキュアストレージが利用できない環境での代替案検討が必要

### 3. パフォーマンス
- セキュアストレージアクセスは非同期処理
- アカウントリストはメモリにキャッシュ

## 今後の改善案

### 1. バックアップ機能
- アカウント情報のエクスポート/インポート
- 複数デバイス間での同期

### 2. セキュリティ強化
- マスターパスワードによる追加保護オプション
- 生体認証との連携強化

### 3. UI/UX改善
- アカウント作成フローの簡素化
- プロフィール編集機能の統合

## まとめ

今回の実装により、kukuriアプリケーションのセキュリティとユーザー体験が大幅に向上しました。ユーザーは安全に複数のNostrアカウントを管理し、素早く切り替えることができるようになりました。

### 主な成果
- ✅ プラットフォーム固有のセキュアストレージ活用
- ✅ 自動ログイン機能による利便性向上
- ✅ 複数アカウントの一元管理
- ✅ 包括的なテストスイートによる品質保証

### 次のステップ
- Phase 2のデータ連携実装に移行
- セキュアストレージのさらなる活用方法の検討
- ユーザーフィードバックに基づく改善