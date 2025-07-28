# エラーハンドリングガイドライン

## 概要

kukuriプロジェクトでは、品質管理とテストの容易性を向上させるため、フロントエンドでの`console.error`の使用を禁止しています。代わりに、カスタムエラーハンドラーを使用してエラーを管理します。

## 背景

- `console.error`はテスト実行時にテストエラーと混同されやすく、品質管理が困難になる
- 環境ごとに適切なエラーハンドリングを行う必要がある
- ユーザーへの通知とログ記録を統一的に管理する必要がある

## エラーハンドラーの使用方法

### 基本的な使用方法

```typescript
import { errorHandler } from '@/lib/errorHandler';

// 基本的なエラーログ
errorHandler.log('エラーメッセージ', error, {
  context: 'ComponentName.methodName',
});

// ユーザーへの通知を含むエラーログ
errorHandler.log('操作に失敗しました', error, {
  context: 'AuthStore.login',
  showToast: true,
  toastTitle: 'ログインに失敗しました',
});
```

### APIとオプション

```typescript
interface ErrorLogOptions {
  showToast?: boolean;      // Toast通知を表示するか
  toastTitle?: string;      // Toast通知のタイトル
  context?: string;         // エラーが発生したコンテキスト
}

// メソッド
errorHandler.log(message: string, error?: unknown, options?: ErrorLogOptions): void
errorHandler.warn(message: string, context?: string): void
errorHandler.info(message: string, context?: string): void
```

## 環境ごとの動作

### テスト環境
- 何も出力しない（テストエラーとの混同を避けるため）

### 開発環境
- `console.warn`を使用してコンソールに出力
- エラーの詳細情報を表示

### 本番環境
- コンソールには出力しない
- 将来的にエラーレポーティングサービス（Sentry等）に送信可能

## 実装例

### ストアでの使用例

```typescript
import { errorHandler } from '@/lib/errorHandler';

// ストア内のアクション
async login(credentials: Credentials) {
  try {
    const result = await api.login(credentials);
    // 成功処理
  } catch (error) {
    errorHandler.log('ログインに失敗しました', error, {
      context: 'AuthStore.login',
      showToast: true,
      toastTitle: 'ログインエラー',
    });
    throw error;
  }
}
```

### コンポーネントでの使用例

```typescript
import { errorHandler } from '@/lib/errorHandler';

function MyComponent() {
  const handleSubmit = async () => {
    try {
      await submitData();
    } catch (error) {
      errorHandler.log('データの送信に失敗しました', error, {
        context: 'MyComponent.handleSubmit',
        showToast: true,
      });
    }
  };
}
```

### イベントリスナーでの使用例

```typescript
import { errorHandler } from '@/lib/errorHandler';

listen<ErrorEvent>('app://error', (event) => {
  errorHandler.log('アプリケーションエラー', event.payload, {
    context: 'EventListener',
    showToast: true,
    toastTitle: 'エラーが発生しました',
  });
});
```

## 禁止事項

以下の使用は禁止されています：

```typescript
// ❌ 禁止
console.error('エラーメッセージ', error);

// ❌ 禁止
try {
  // 処理
} catch (error) {
  console.error(error);
}
```

## 例外

以下の場合のみ、標準エラー出力の使用が許可されています：

1. **E2Eテストの設定エラー**
   - `process.stderr.write()`を使用
   - Tauriドライバーのインストール確認など、テスト環境のセットアップエラー

2. **ビルドツールの設定**
   - webpackやviteの設定ファイル内でのエラー出力

## テストでの考慮事項

### 統合テスト

統合テストでは、エラーハンドリングはストアやコンポーネント内で行われるため、テストコード内でconsole.errorを使用する必要はありません：

```typescript
// テストコンポーネント
const handleAction = async () => {
  try {
    await someAction();
  } catch (error) {
    // エラーはストア内で処理される
    // テストコードでは何もしない
  }
};
```

### ユニットテスト

エラーハンドラーのモックが必要な場合：

```typescript
import { vi } from 'vitest';
import { errorHandler } from '@/lib/errorHandler';

// モック化
vi.mock('@/lib/errorHandler', () => ({
  errorHandler: {
    log: vi.fn(),
    warn: vi.fn(),
    info: vi.fn(),
  },
}));

// テスト内で検証
expect(errorHandler.log).toHaveBeenCalledWith(
  'エラーメッセージ',
  expect.any(Error),
  expect.objectContaining({ context: 'TestContext' })
);
```

## 移行ガイド

既存のconsole.error使用箇所を置き換える手順：

1. `errorHandler`をインポート
2. `console.error`を`errorHandler.log`に置き換え
3. 適切なコンテキスト情報を追加
4. 必要に応じてtoast通知オプションを追加

```typescript
// Before
console.error('Failed to fetch data:', error);

// After
errorHandler.log('Failed to fetch data', error, {
  context: 'DataService.fetch',
  showToast: true,
  toastTitle: 'データ取得エラー',
});
```

## 今後の拡張

将来的に以下の機能を追加予定：

- エラーレポーティングサービスとの統合（Sentry、LogRocket等）
- エラーの分類とフィルタリング
- エラーレートの監視とアラート
- ユーザーフィードバックの収集