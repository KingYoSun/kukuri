# Zustand テスティングベストプラクティス

最終更新: 2025年7月28日

## 概要

このドキュメントでは、kukuriプロジェクトにおけるZustandストアのテスト実装のベストプラクティスを説明します。

## 基本的なテスト構造

### 1. ストアのリセット

各テストの前後でストアを初期状態にリセットします。

```typescript
import { useAuthStore } from '@/stores/authStore';

describe('AuthStore', () => {
  beforeEach(() => {
    // ストアを初期状態にリセット
    useAuthStore.setState({
      currentUser: null,
      accounts: [],
      isInitializing: false,
      isInitialized: false,
    });
  });

  afterEach(() => {
    vi.clearAllMocks();
  });
});
```

### 2. 状態の変更テスト

```typescript
it('ユーザーのログインが正しく処理されること', async () => {
  const mockUser = {
    id: 'test123',
    pubkey: 'pubkey123',
    npub: 'npub123',
    name: 'テストユーザー',
    displayName: 'テストユーザー',
    picture: '',
    about: '',
    nip05: '',
  };

  // アクションを実行
  const { setCurrentUser } = useAuthStore.getState();
  setCurrentUser(mockUser);

  // 状態を確認
  const { currentUser } = useAuthStore.getState();
  expect(currentUser).toEqual(mockUser);
});
```

## コンポーネントでのテスト

### 1. ストアを使用するコンポーネントのテスト

```typescript
import { render, screen } from '@testing-library/react';
import { Header } from '@/components/layout/Header';
import { useAuthStore } from '@/stores/authStore';

describe('Header', () => {
  beforeEach(() => {
    // テスト用のユーザーを設定
    useAuthStore.setState({
      currentUser: {
        id: 'test123',
        pubkey: 'pubkey123',
        npub: 'npub123',
        name: 'テストユーザー',
        displayName: 'テストユーザー',
        picture: '',
        about: '',
        nip05: '',
      },
    });
  });

  it('ユーザー情報が表示されること', () => {
    render(<Header />);
    expect(screen.getByText('テ')).toBeInTheDocument(); // アバターの初期文字
  });
});
```

### 2. 非同期アクションのテスト

```typescript
it('非同期ログイン処理が正しく動作すること', async () => {
  const mockLoginResponse = {
    public_key: 'pubkey123',
    npub: 'npub123',
  };

  // APIモックの設定
  vi.mocked(invoke).mockResolvedValueOnce(mockLoginResponse);

  // ログインを実行
  const { loginWithNsec } = useAuthStore.getState();
  await loginWithNsec('nsec1...', true);

  // 結果を確認
  const { currentUser } = useAuthStore.getState();
  expect(currentUser).toBeDefined();
  expect(currentUser?.npub).toBe('npub123');
});
```

## エラーハンドリングのテスト

### 1. errorHandlerとの統合

```typescript
import { errorHandler } from '@/lib/errorHandler';

// errorHandlerをモック
vi.mock('@/lib/errorHandler', () => ({
  errorHandler: {
    log: vi.fn(),
    warn: vi.fn(),
    info: vi.fn(),
  },
}));

it('エラー時にerrorHandlerが呼ばれること', async () => {
  // エラーレスポンスを設定
  vi.mocked(invoke).mockRejectedValueOnce(new Error('Login failed'));

  const { loginWithNsec } = useAuthStore.getState();
  await loginWithNsec('invalid_nsec', false);

  // errorHandlerが呼ばれたことを確認
  expect(errorHandler.log).toHaveBeenCalledWith(
    'Login failed',
    expect.any(Error),
    expect.objectContaining({
      context: 'authStore.loginWithNsec',
    })
  );
});
```

## React Testing Libraryとの統合

### 1. カスタムレンダラーの使用

複数のプロバイダーが必要な場合のカスタムレンダラー：

```typescript
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render } from '@testing-library/react';

const createTestQueryClient = () => new QueryClient({
  defaultOptions: {
    queries: { retry: false },
    mutations: { retry: false },
  },
});

export function renderWithProviders(
  ui: React.ReactElement,
  options?: RenderOptions
) {
  const queryClient = createTestQueryClient();
  
  return render(
    <QueryClientProvider client={queryClient}>
      {ui}
    </QueryClientProvider>,
    options
  );
}
```

### 2. waitForを使った非同期テスト

```typescript
import { waitFor } from '@testing-library/react';

it('非同期データが表示されること', async () => {
  render(<MyComponent />);
  
  // データが表示されるまで待機
  await waitFor(() => {
    expect(screen.getByText('データ')).toBeInTheDocument();
  });
});
```

## よくある問題と解決策

### 1. ストアの状態が他のテストに影響する

**問題**: あるテストでの状態変更が他のテストに影響する

**解決策**: 各テストの前にストアをリセット
```typescript
beforeEach(() => {
  // すべてのストアをリセット
  useAuthStore.setState(initialAuthState);
  useUIStore.setState(initialUIState);
  useTopicStore.setState(initialTopicState);
});
```

### 2. モックが正しく動作しない

**問題**: Zustandストアのアクションがモックされない

**解決策**: ストアのgetStateを使用してアクションをテスト
```typescript
// ❌ 間違い
const mockLogin = vi.fn();
useAuthStore.mockReturnValue({ login: mockLogin });

// ✅ 正しい
const { login } = useAuthStore.getState();
const spy = vi.spyOn(useAuthStore.getState(), 'login');
```

### 3. 非同期アクションのタイミング

**問題**: 非同期アクションの完了を待たずにアサーションが実行される

**解決策**: async/awaitとwaitForを適切に使用
```typescript
it('非同期処理のテスト', async () => {
  const { fetchData } = useMyStore.getState();
  
  // アクションを実行
  await fetchData();
  
  // または、UIの更新を待つ
  await waitFor(() => {
    const { data } = useMyStore.getState();
    expect(data).toBeDefined();
  });
});
```

## パフォーマンスの考慮

### 1. 選択的な購読

コンポーネントで必要な部分だけを購読：

```typescript
// ❌ ストア全体を購読
const store = useAuthStore();

// ✅ 必要な部分だけを購読
const currentUser = useAuthStore((state) => state.currentUser);
const isAuthenticated = useAuthStore((state) => !!state.currentUser);
```

### 2. メモ化の活用

複雑な導出状態はメモ化：

```typescript
const useIsAdmin = () => {
  return useAuthStore((state) => 
    useMemo(() => state.currentUser?.role === 'admin', [state.currentUser])
  );
};
```

## 参考資料

- [Zustand公式テストガイド](https://zustand.docs.pmnd.rs/guides/testing)
- [React Testing Library](https://testing-library.com/docs/react-testing-library/intro/)
- プロジェクト内の実装例: `src/__tests__/stores/`, `src/stores/__tests__/`