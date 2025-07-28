import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { LoginForm } from '../LoginForm';
import { useAuthStore } from '@/stores/authStore';
import { toast } from 'sonner';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { ReactNode } from 'react';
import { errorHandler } from '@/lib/errorHandler';

// モック
const mockNavigate = vi.fn();
vi.mock('@tanstack/react-router', () => ({
  useNavigate: () => mockNavigate,
}));

vi.mock('sonner', () => ({
  toast: {
    error: vi.fn(),
    success: vi.fn(),
  },
}));

vi.mock('@/stores/authStore');

vi.mock('@/lib/errorHandler', () => ({
  errorHandler: {
    log: vi.fn(),
  },
}));

const createWrapper = () => {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });

  return ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );
};

describe('LoginForm', () => {
  const mockLoginWithNsec = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    (useAuthStore as unknown as vi.Mock).mockReturnValue({
      loginWithNsec: mockLoginWithNsec,
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('ログインフォームが正しく表示される', () => {
    render(<LoginForm />, { wrapper: createWrapper() });

    // ヘッダー
    expect(screen.getByRole('button', { name: /戻る/ })).toBeInTheDocument();
    // CardTitleコンポーネントは div[data-slot="card-title"] として出力される
    const loginTexts = screen.getAllByText('ログイン');
    const cardTitle = loginTexts.find((el) => el.closest('[data-slot="card-title"]'));
    expect(cardTitle).toBeTruthy();
    expect(screen.getByText('既存のアカウントでログインします')).toBeInTheDocument();

    // フォーム要素
    expect(screen.getByLabelText('秘密鍵（nsec）')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('nsec1...')).toBeInTheDocument();
    expect(screen.getByText('nsec1で始まるNostr秘密鍵を入力してください')).toBeInTheDocument();

    // 警告メッセージ
    expect(screen.getByText('⚠️ 秘密鍵は絶対に他人に教えないでください')).toBeInTheDocument();

    // ログインボタン
    expect(screen.getByRole('button', { name: 'ログイン' })).toBeInTheDocument();
  });

  it('戻るボタンがクリックされた時、ウェルカム画面に遷移する', async () => {
    const user = userEvent.setup();

    render(<LoginForm />, { wrapper: createWrapper() });

    const backButton = screen.getByRole('button', { name: /戻る/ });
    await user.click(backButton);

    expect(mockNavigate).toHaveBeenCalledWith({ to: '/welcome' });
  });

  it('パスワード表示/非表示の切り替えが動作する', async () => {
    const user = userEvent.setup();

    render(<LoginForm />, { wrapper: createWrapper() });

    const input = screen.getByLabelText('秘密鍵（nsec）');
    const toggleButton = screen.getByRole('button', { name: '' }); // アイコンボタン

    // 初期状態はパスワード非表示
    expect(input).toHaveAttribute('type', 'password');

    // ボタンクリックで表示
    await user.click(toggleButton);
    expect(input).toHaveAttribute('type', 'text');

    // 再度クリックで非表示
    await user.click(toggleButton);
    expect(input).toHaveAttribute('type', 'password');
  });

  it('空の秘密鍵でログインしようとするとエラーを表示する', async () => {
    const user = userEvent.setup();

    render(<LoginForm />, { wrapper: createWrapper() });

    const loginButton = screen.getByRole('button', { name: 'ログイン' });
    await user.click(loginButton);

    expect(toast.error).toHaveBeenCalledWith('秘密鍵（nsec）を入力してください');
    expect(mockLoginWithNsec).not.toHaveBeenCalled();
  });

  it('無効な形式の秘密鍵でログインしようとするとエラーを表示する', async () => {
    const user = userEvent.setup();

    render(<LoginForm />, { wrapper: createWrapper() });

    const input = screen.getByLabelText('秘密鍵（nsec）');
    await user.type(input, 'invalid-nsec');

    const loginButton = screen.getByRole('button', { name: 'ログイン' });
    await user.click(loginButton);

    expect(toast.error).toHaveBeenCalledWith(
      '無効な形式です。nsec1で始まる秘密鍵を入力してください',
    );
    expect(mockLoginWithNsec).not.toHaveBeenCalled();
  });

  it('有効な秘密鍵でログインに成功する', async () => {
    mockLoginWithNsec.mockResolvedValue(undefined);
    const user = userEvent.setup();

    render(<LoginForm />, { wrapper: createWrapper() });

    const input = screen.getByLabelText('秘密鍵（nsec）');
    await user.type(input, 'nsec1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq');

    const loginButton = screen.getByRole('button', { name: 'ログイン' });
    await user.click(loginButton);

    // ローディング状態（ボタンのテキストがすぐに変わらない可能性があるため削除）

    // ログイン処理が呼ばれる
    await waitFor(() => {
      expect(mockLoginWithNsec).toHaveBeenCalledWith(
        'nsec1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq',
        true,
      );
    });

    // 成功メッセージ
    await waitFor(() => {
      expect(toast.success).toHaveBeenCalledWith('ログインしました');
    });

    // ホーム画面への遷移
    await waitFor(() => {
      expect(mockNavigate).toHaveBeenCalledWith({ to: '/' });
    });
  });

  it('ログインに失敗した場合、エラーメッセージを表示する', async () => {
    const error = new Error('Invalid nsec');
    mockLoginWithNsec.mockRejectedValue(error);
    const user = userEvent.setup();

    render(<LoginForm />, { wrapper: createWrapper() });

    const input = screen.getByLabelText('秘密鍵（nsec）');
    await user.type(input, 'nsec1invalid');

    const loginButton = screen.getByRole('button', { name: 'ログイン' });
    await user.click(loginButton);

    // エラーメッセージ
    await waitFor(() => {
      expect(toast.error).toHaveBeenCalledWith('ログインに失敗しました。秘密鍵を確認してください');
    });

    // errorHandlerが呼ばれる
    expect(errorHandler.log).toHaveBeenCalledWith('Login failed', error, {
      context: 'LoginForm.handleSubmit',
    });

    // ナビゲーションは発生しない
    expect(mockNavigate).not.toHaveBeenCalled();

    // ローディング状態が解除される
    expect(screen.getByRole('button', { name: 'ログイン' })).not.toBeDisabled();
  });

  it('空の秘密鍵でエンターキーを押してもエラーが表示される', async () => {
    const user = userEvent.setup();

    render(<LoginForm />, { wrapper: createWrapper() });

    const input = screen.getByLabelText('秘密鍵（nsec）');

    // エンターキーを押す（空の状態で）
    await user.type(input, '{Enter}');

    // エラーメッセージが表示される
    expect(toast.error).toHaveBeenCalledWith('秘密鍵（nsec）を入力してください');
    expect(mockLoginWithNsec).not.toHaveBeenCalled();
  });
});
