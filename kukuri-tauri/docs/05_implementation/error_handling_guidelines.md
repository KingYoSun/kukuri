# エラーハンドリングガイドライン

最終更新: 2025年7月28日

## 概要

kukuriプロジェクトでは、統一されたエラーハンドリングを実現するため、`errorHandler`ユーティリティを使用します。

## 基本ルール

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

## 今後の拡張予定

1. **エラーレポーティング**: Sentry等の外部サービスとの連携
2. **エラー分析**: エラーパターンの自動検出と通知
3. **ユーザーフィードバック**: エラー発生時のフィードバック収集機能