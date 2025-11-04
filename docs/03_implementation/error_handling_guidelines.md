# エラーハンドリングガイドライン

最終更新: 2025年10月18日

## 概要

kukuriプロジェクトでは、統一されたエラーハンドリングを実現するため、`errorHandler`ユーティリティを使用します。

## 基本ルール

### ESLintによる強制
`eslint.config.js` では `no-console` ルールを有効化し、`console.warn` / `console.info` のみ使用可能としました。開発・テストでのログは必要に応じて `errorHandler.info` / `errorHandler.warn` を利用してください。

```javascript
// eslint.config.js（抜粋）
'no-console': ['error', { allow: ['warn', 'info'] }],
```

### ❌ 使用禁止
```typescript
// フロントエンドでconsole.errorの使用は禁止
console.error('エラーが発生しました', error);
```

### ✅ 推奨される使い方
```typescript
import { errorHandler } from '@/lib/errorHandler';

// エラーログの記録
errorHandler.log('エラーが発生しました', error, {
  context: 'ComponentName.methodName',
  showToast: true,
  toastTitle: '処理に失敗しました'
});
```

## errorHandlerの機能

### 1. 基本的なメソッド

#### log(message, error?, options?)
エラーログを記録します。

```typescript
errorHandler.log('API呼び出しに失敗', error, {
  context: 'PostService.create',
  showToast: true,
  toastTitle: '投稿の作成に失敗しました'
});
```

#### warn(message, context?)
警告ログを記録します。

```typescript
errorHandler.warn('非推奨のAPIを使用しています', 'AuthService');
```

#### info(message, context?)
情報ログを記録します。

```typescript
errorHandler.info('キャッシュをクリアしました', 'CacheService');
```

### 2. 環境に応じた動作

- **開発環境**: コンソールにログを出力
- **本番環境**: コンソール出力を抑制（将来的に外部サービスへ送信）
- **テスト環境**: すべてのログを抑制

### 3. ユーザーへの通知

`showToast`オプションを使用して、ユーザーにエラーを通知できます。

```typescript
errorHandler.log('ネットワークエラー', error, {
  showToast: true,
  toastTitle: '接続エラー'
});
```

## テスト環境での使用

### テスト用メソッド

テスト環境では、`setTestEnvironment`メソッドを使用して環境を制御できます。

```typescript
import { ErrorHandler } from '@/lib/errorHandler';

describe('MyComponent', () => {
  let errorHandler: ErrorHandler;

  beforeEach(() => {
    errorHandler = new ErrorHandler();
    // 開発環境として動作させる（ログを出力）
    errorHandler.setTestEnvironment('development');
  });

  it('エラーログが出力されること', () => {
    const consoleSpy = vi.spyOn(console, 'warn');
    errorHandler.log('テストエラー');
    expect(consoleSpy).toHaveBeenCalled();
  });
});
```

### モックの設定

テストでは通常、errorHandlerをモックします。

```typescript
vi.mock('@/lib/errorHandler', () => ({
  errorHandler: {
    log: vi.fn(),
    warn: vi.fn(),
    info: vi.fn(),
  },
}));
```

## ベストプラクティス

### 1. コンテキストの提供

エラーがどこで発生したかを明確にするため、常にコンテキストを提供します。

```typescript
errorHandler.log('データの取得に失敗', error, {
  context: 'useTopics.fetchTopics'
});
```

### 2. ユーザーフレンドリーなメッセージ

ユーザーに表示するメッセージは、技術的な詳細を避けて分かりやすくします。

```typescript
errorHandler.log(
  'Failed to parse JSON response', // 開発者向けの詳細
  error,
  {
    showToast: true,
    toastTitle: '情報の読み込みに失敗しました' // ユーザー向けメッセージ
  }
);
```

### 3. エラーの伝播

必要に応じてエラーを再スローします。

```typescript
try {
  await someAsyncOperation();
} catch (error) {
  errorHandler.log('操作に失敗', error, {
    context: 'MyService.operation',
    showToast: true
  });
  throw error; // 上位でも処理が必要な場合
}
```

## マイグレーションガイド

既存のconsole.error使用箇所を置き換える場合：

```typescript
// Before
try {
  await api.call();
} catch (error) {
  console.error('API call failed:', error);
}

// After
try {
  await api.call();
} catch (error) {
  errorHandler.log('API call failed', error, {
    context: 'MyComponent.apiCall'
  });
}
```

## UIコンポーネント別エラーハンドリングキー（2025年11月04日追加）

| キー | 想定コンポーネント / コンテキスト | 推奨 `toastTitle` | 備考 |
| --- | --- | --- | --- |
| `TrendingFeed.fetch_failed` | `/trending` ルートの `TrendingRoute` / `TrendingTopicList` | `トレンド情報の取得に失敗しました` | `context: 'TrendingRoute.load'`。再試行ボタン押下時は `errorHandler.log` に `retry: true` を付加。 |
| `FollowingFeed.fetch_failed` | `/following` ルートの `FollowingFeed` | `フォロー中の最新投稿を読み込めませんでした` | 429（レート制限）は `retryAfterSeconds` を `meta` に含め、UI でカウントダウン表示。 |
| `UserSearch.invalid_query` | `/search` (users) の `UserSearchResults` | `2文字以上入力してください` | クエリ長が足りない場合に表示。入力欄へエラースタイルを適用し、既存結果は保持。 |
| `UserSearch.fetch_failed` | `UserSearchResults` / `useUserSearchQuery` | `ユーザー検索に失敗しました` | ネットワーク/サーバーエラー時。`context` に `statusCode` を含める。 |
| `UserSearch.rate_limited` | 同上 | `一定時間後に再試行してください` | `meta.retryAfter`（秒）を UI に表示し、再試行ボタンを無効化。 |

> 新しいキーを追加する場合は、`docs/01_project/activeContext/artefacts/phase5_user_flow_inventory.md` の該当節と照合し、命名の一貫性を保つ。

## 今後の拡張予定

## バックエンド（Tauri/Rust）との連携

### AppError の共通化
Rust 側では `thiserror` を用いた `AppError` 列挙体でドメインエラーを一元管理します。各バリアントには `code()`（機械判定用）と `user_message()`（UI表示用）を実装しています。

```rust
#[derive(Debug, Error)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(String),
    // ...
}

impl AppError {
    pub fn code(&self) -> &'static str { /* DATABASE_ERROR など */ }
    pub fn user_message(&self) -> String { /* ユーザー向け日本語メッセージ */ }
}
```

### ApiResponse の拡張
すべての Tauri コマンドは `ApiResponse<T>` を返し、成功/失敗を統一形式で扱います。失敗時は `error` にユーザー向けメッセージ、`error_code` に `AppError::code()` を格納します。

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub error_code: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn from_result(result: crate::shared::Result<T>) -> Self {
        match result {
            Ok(data) => Self::success(data),
            Err(err) => Self::from_app_error(err),
        }
    }
}
```

フロントエンドで `invoke` を用いる際は、`success` / `error_code` を参照し、`errorHandler` でログと通知を行ってください。

1. **エラーレポーティング**: Sentry等の外部サービスとの連携
2. **エラー分析**: エラーパターンの自動検出と通知
3. **ユーザーフィードバック**: エラー発生時のフィードバック収集機能
