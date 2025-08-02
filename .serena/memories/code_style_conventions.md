# コードスタイルと規約

## 基本原則
- **言語**: ドキュメントとコメントは必ず日本語で記述
- **DRY原則**: 新しいクラス・メソッド・型を実装する際は、同じ機能が既にないか必ず調査
- **依存最新化**: 依存ライブラリを追加する際は、webから最新バージョンを確認

## TypeScript/React規約

### 命名規則
- **コンポーネント**: PascalCase（例: `PostCard`, `TopicSelector`）
- **フック**: camelCaseで`use`プレフィックス（例: `useAuth`, `useTopics`）
- **変数・関数**: camelCase（例: `handleSubmit`, `isLoading`）
- **定数**: UPPER_SNAKE_CASE（例: `MAX_POST_LENGTH`）
- **型・インターフェース**: PascalCase（例: `Post`, `Topic`）

### ファイル構成
```
src/
├── components/       # UIコンポーネント
│   └── auth/        # 機能別にグループ化
├── hooks/           # カスタムフック
├── stores/          # Zustandストア
├── lib/             # ユーティリティ関数
├── types/           # 型定義
└── routes/          # ページコンポーネント
```

### コンポーネント実装例
```typescript
import { FC } from 'react';
import { Button } from '@/components/ui/button';

interface PostCardProps {
  post: Post;
  onLike?: (postId: string) => void;
}

export const PostCard: FC<PostCardProps> = ({ post, onLike }) => {
  // フックは最上部に配置
  const { user } = useAuth();
  
  // イベントハンドラー
  const handleLike = () => {
    onLike?.(post.id);
  };
  
  return (
    <Card>
      {/* 実装 */}
    </Card>
  );
};
```

### エラーハンドリング
```typescript
// ❌ 禁止: console.errorの直接使用
console.error('エラーが発生しました', error);

// ✅ 推奨: errorHandlerを使用
import { errorHandler } from '@/lib/errorHandler';

errorHandler.log('API呼び出しに失敗', error, {
  context: 'PostService.create',
  showToast: true,
  toastTitle: '投稿の作成に失敗しました'
});
```

## Rust規約

### 命名規則
- **関数・変数**: snake_case（例: `generate_keypair`, `user_id`）
- **構造体・列挙型**: PascalCase（例: `KeyManager`, `EventType`）
- **定数**: UPPER_SNAKE_CASE（例: `MAX_CONNECTIONS`）
- **モジュール**: snake_case（例: `auth`, `p2p`）

### エラーハンドリング
- `Result<T, E>`を使用してエラーを明示的に扱う
- `anyhow`でエラーコンテキストを追加
- `thiserror`でカスタムエラー型を定義

### モジュール構成
```
src/
├── modules/
│   ├── auth/        # 認証関連
│   ├── crypto/      # 暗号化
│   ├── database/    # DB操作
│   ├── p2p/         # P2P通信
│   └── mod.rs       # モジュール定義
└── main.rs
```

## テスト規約

### フロントエンド
- コンポーネントごとにテストファイルを作成
- `@testing-library/react`を使用
- ユーザー操作を中心にテスト

### バックエンド
- モジュールごとに`#[cfg(test)]`ブロックでテスト
- `tempfile`でテスト用の一時ファイルを使用
- 非同期テストは`#[tokio::test]`を使用

## Zustandストアのテスト
`docs/03_implementation/zustand_testing_best_practices.md`を必ず参照

## コミット規約
- 明確で簡潔なメッセージ
- 日本語で記述
- 形式: `<type>: <description>`
  - feat: 新機能
  - fix: バグ修正
  - docs: ドキュメント
  - test: テスト
  - refactor: リファクタリング