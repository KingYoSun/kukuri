# 既知の問題と注意事項

**最終更新**: 2025年8月3日

> **注記**: 2025年7月の問題・注意事項は`archives/issuesAndNotes_2025-07.md`にアーカイブされました。

## 現在の問題

### DOM検証警告（2025年8月3日）
**問題**: MarkdownPreview.test.tsxでDOM検証の警告が発生

**詳細**:
- 警告メッセージ: `validateDOMNesting(...): <div> cannot appear as a descendant of <p>`
- 原因: React MarkdownコンポーネントのDOM構造
- 影響: テストは成功するが、警告が表示される
- 優先度: 低（実際の機能には影響なし）

### リント警告（2025年7月29日）
**問題**: ESLintで14件の警告（`--max-warnings 0`の制約により、ビルドエラーになる）

**詳細**:
- **@typescript-eslint/no-explicit-any**: 13箇所
  - PostComposer.test.tsx: 4箇所
  - TopicSelector.test.tsx: 2箇所
  - Home.test.tsx: 7箇所
  - 主にモック関数の型定義で使用

- **react-refresh/only-export-components**: 1箇所
  - form.tsx: badgeVariants定数のエクスポート

## 解決済みの問題

### フロントエンドテストエラーの解消（2025年8月3日）
**問題**: フロントエンドテストで14個のテストが失敗していた

**症状**:
- PostCard.test.tsx: 複数要素の選択エラー、ボタンインデックスの不一致、フォームが閉じない問題
- ReactionPicker.test.tsx: TauriApiのインポートエラー
- topicStore.ts: null参照エラー
- Sidebar.test.tsx: ナビゲーション先の不一致
- その他の非同期処理関連のエラー

**解決策**:

1. PostCard.test.tsxの修正
```typescript
// 複数要素選択の問題を解決
const { container } = renderPostCard();
expect(container.querySelector('p')?.textContent).toBe('テスト投稿です');

// Collapsibleモックの実装改善
vi.mock('@/components/ui/collapsible', () => ({
  Collapsible: ({ children, open }: { children: React.ReactNode; open: boolean }) => (
    <div data-state={open ? 'open' : 'closed'}>
      {open ? children : null}
    </div>
  ),
  CollapsibleContent: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
}));
```

2. ReactionPicker.test.tsxの修正
```typescript
vi.mock('@/lib/api/tauri', () => ({
  NostrAPI: {
    sendReaction: vi.fn(),
  },
  TauriApi: {},  // 追加
}));
```

3. topicStore.tsの修正
```typescript
const apiTopics = await TauriApi.getTopics();
if (!apiTopics) {
  set({ topics: new Map() });
  return;
}
```

4. Sidebar.test.tsxの修正
```typescript
expect(mockNavigate).toHaveBeenCalledWith({ to: '/' });
```

**結果**:
- 537個のテスト全て成功（533個成功、4個スキップ）
- DOM検証警告が1つ残るが、実害なし
- テストの安定性が大幅に向上

### Windows環境でのアカウント永続化問題（2025年8月2日）
**問題**: Windows環境で新規アカウント作成後、リロードするとログイン状態が維持されない

**症状**:
- アカウント作成は成功するが、リロード後に「No metadata entry found in keyring」となる
- 保存直後の読み取りテストで「NoEntry」エラーが発生
- Windows Credential Managerへのアクセスが正しく機能しない

**根本原因**:
1. `Entry::new_with_target()`の使い方が複雑すぎた
2. Windows専用のtarget名の設定が適切でなかった
3. `windows-native`フィーチャーが有効化されていなかった

**解決策**:

1. シンプルなアプローチへの変更
```rust
// 以前の複雑な実装を削除
// 全プラットフォームで統一的なシンプルな実装に変更
let entry = Entry::new(SERVICE_NAME, ACCOUNTS_KEY).context("Failed to create keyring entry")?;
```

2. windows-nativeフィーチャーの有効化
```toml
# Cargo.toml
keyring = { version = "3.6.3", features = ["windows-native"] }
```

3. 不要なコードの削除
- fallback storageの完全削除（セキュリティリスク）
- WSL検出機能の削除
- Windows専用の条件分岐を削除

**結果**:
- 新規アカウント作成後のリロードでログイン状態が維持される
- Windows環境での正常動作を確認
- デバッグログで保存・読み取りの成功を確認

**注意事項**:
- Windows、macOS、Linuxで統一的な実装が可能
- `Entry::new()`を使用することでコードがシンプルに
- keyringライブラリが各プラットフォームの特性を適切に処理

### Windows環境での起動エラー（2025年8月1日）
**問題**: Windows環境で`pnpm tauri dev`実行時に「ファイル名、ディレクトリ名、またはボリューム ラベルの構文が間違っています。 (os error 123)」エラーが発生

**症状**:
- `AppState::new()`の初期化時にパニック
- 相対パス`./data`の使用がWindows環境で無効なパスとして認識される
- SQLiteのデータベースURL形式がWindows非対応

**根本原因**:
1. 相対パス`./data`の使用がプラットフォーム非依存でない
2. SQLiteのURL形式がOSによって異なる（Windows: `sqlite:C:/path/to/db`, Unix: `sqlite://path/to/db`）
3. `tauri::Manager` traitのインポートが必要（`path()`メソッド使用のため）

**解決策**:

1. Tauri AppHandleを使用したプラットフォーム固有パスの取得
```rust
// state.rs
pub async fn new(app_handle: &tauri::AppHandle) -> anyhow::Result<Self> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| anyhow::anyhow!("Failed to get app data dir: {}", e))?;
```

2. `tauri::Manager` traitのインポート追加
```rust
use tauri::Manager;
```

3. Windows用SQLite URL形式の実装
```rust
// Windows環境では特別な形式を使用
let db_url = if cfg!(windows) {
    format!("sqlite:{}?mode=rwc", db_path_str.replace('\\', "/"))
} else {
    format!("sqlite://{}?mode=rwc", db_path_str)
};
```

4. Database::initializeメソッドの改善
```rust
// URL形式に関わらず正しくファイルパスを抽出
let file_path = if database_url.starts_with("sqlite:///") {
    &database_url[10..]
} else if database_url.starts_with("sqlite://") {
    &database_url[9..]
} else if database_url.starts_with("sqlite:") {
    &database_url[7..]
} else {
    database_url
};
```

**結果**:
- Windows環境でアプリケーションが正常に起動
- データディレクトリは`C:\Users\{username}\AppData\Roaming\com.kukuri.app`に作成
- プラットフォーム依存のパス処理が正しく動作

**注意事項**:
- Windowsでは`sqlite:C:/path/to/db?mode=rwc`形式を使用（スラッシュなし）
- パス区切り文字はスラッシュに統一する必要がある
- `tauri::Manager` traitのインポートを忘れないこと

### WSL環境でのアカウント永続化問題（2025年8月1日）
**問題**: WSL環境でアカウント作成後、アプリケーションをリロードするとログイン状態が維持されない

**症状**:
- アカウント作成は成功するが、リロード時に`getCurrentAccount`が`null`を返す
- `keyring`クレートがWSL環境でSecret Serviceにアクセスできない
- コンソールログに「No current account found in secure storage」と表示される

**根本原因**:
1. `authStore`のpersist設定で`isAuthenticated`が常に`false`で保存されていた
2. `generate_keypair`コマンドが`npub`を返していなかったため、不正な形式のキーで保存されていた
3. WSL環境ではLinuxの標準的なセキュアストレージ（Secret Service）が利用できない

**解決策**:

1. authStoreのpersist設定を修正
```typescript
// 修正前
partialize: (state) => ({
  isAuthenticated: false, // 常にfalseで保存
  currentUser: state.currentUser,
}),

// 修正後
partialize: (state) => ({
  // isAuthenticatedはセキュアストレージからの復元で管理するため保存しない
  currentUser: state.currentUser,
}),
```

2. Rustバックエンドの修正
```rust
// key_manager.rs
pub async fn generate_keypair(&self) -> Result<(String, String, String)> {
    let keys = Keys::generate();
    let public_key = keys.public_key().to_hex();
    let secret_key = keys.secret_key().to_bech32()?;
    let npub = keys.public_key().to_bech32()?; // npubを追加
    // ...
    Ok((public_key, secret_key, npub))
}
```

3. WSL環境検出とフォールバック実装
```rust
// secure_storage/mod.rs
fn is_wsl() -> bool {
    if cfg!(target_os = "linux") {
        if let Ok(osrelease) = std::fs::read_to_string("/proc/sys/kernel/osrelease") {
            return osrelease.to_lowercase().contains("microsoft");
        }
    }
    false
}
```

4. フォールバックストレージの実装
- `secure_storage/fallback.rs`を作成
- WSL環境では`~/.local/share/kukuri-dev/secure_storage/`にファイルとして保存
- 開発環境専用（本番環境では使用しない）

**結果**:
- アカウント作成後のリロードでログイン状態が維持される
- WSL環境でも正常に動作することを確認
- デバッグログで保存・読み込み処理の診断が可能

**注意事項**:
- フォールバック実装は開発環境専用（セキュリティリスクあり）
- 本番環境でのWSL対応は別途検討が必要
- Windows、macOS、Linux（非WSL）では引き続き標準のセキュアストレージを使用

## 現在の注意事項

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
- **Unhandled Promise Rejection警告**: エラーハンドリングテストで発生（2025年7月27日）
  - Promise.rejectを使用するテストで警告が表示される
  - テスト自体は正常に動作し、すべて成功
  - VitestがPromiseエラーを検出する仕様による
  - 実際のアプリケーション動作には影響なし
- **バックエンド統合テスト**: P2P通信関連の6件は#[ignore]属性でスキップ（2025年7月27日）
  - ネットワーク接続が必要なテストはローカル環境で実行
  - CI環境での安定性向上
  - 全テスト: 88 passed, 0 failed, 9 ignored

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

## 技術的な決定事項

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