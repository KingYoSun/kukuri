import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { WelcomeScreen } from '@/components/auth/WelcomeScreen';
import { useAuthStore } from '@/stores/authStore';
import { toast } from 'sonner';
import { errorHandler } from '@/lib/errorHandler';

// モック
const mockNavigate = vi.fn();
vi.mock('@tanstack/react-router', () => ({
  useNavigate: () => mockNavigate,
}));

vi.mock('sonner', () => ({
  toast: {
    error: vi.fn(),
  },
}));

vi.mock('@/stores/authStore');

vi.mock('@/lib/errorHandler', () => ({
  errorHandler: {
    log: vi.fn(),
    info: vi.fn(),
  },
}));

describe('WelcomeScreen', () => {
  const mockGenerateNewKeypair = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    (useAuthStore as unknown as vi.Mock).mockReturnValue({
      generateNewKeypair: mockGenerateNewKeypair,
    });
    (useAuthStore as unknown as vi.Mock).getState = vi.fn(() => ({
      isLoggedIn: false,
      currentUser: null,
    }));
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('ウェルカム画面が正しく表示される', () => {
    render(<WelcomeScreen />);

    // タイトルとサブタイトル
    expect(screen.getByText('kukuriへようこそ')).toBeInTheDocument();
    expect(screen.getByText('分散型トピック中心ソーシャルアプリケーション')).toBeInTheDocument();

    // 特徴の説明
    expect(screen.getByText('・トピックベースのタイムラインで情報を共有')).toBeInTheDocument();
    expect(screen.getByText('・P2Pネットワークによる検閲耐性')).toBeInTheDocument();
    expect(screen.getByText('・Nostrプロトコルによる分散型アーキテクチャ')).toBeInTheDocument();

    // ボタン
    expect(screen.getByRole('button', { name: '新規アカウント作成' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '既存アカウントでログイン' })).toBeInTheDocument();
  });

  it('新規アカウント作成ボタンがクリックされた時、鍵ペアを生成してプロフィール設定画面に遷移する', async () => {
    mockGenerateNewKeypair.mockResolvedValue({ nsec: 'nsec1test...' });
    const user = userEvent.setup();

    render(<WelcomeScreen />);

    const createButton = screen.getByRole('button', { name: '新規アカウント作成' });
    await user.click(createButton);

    // 鍵ペア生成が呼ばれる
    await waitFor(() => {
      expect(mockGenerateNewKeypair).toHaveBeenCalledTimes(1);
      expect(mockGenerateNewKeypair).toHaveBeenCalledWith(true, { deferInitialization: true });
    });

    // プロフィール設定画面への遷移
    await waitFor(() => {
      expect(mockNavigate).toHaveBeenCalledWith({ to: '/profile-setup' });
    });
  });

  it('新規アカウント作成でエラーが発生した場合、エラートーストを表示する', async () => {
    const error = new Error('Failed to generate keypair');
    mockGenerateNewKeypair.mockRejectedValue(error);
    const user = userEvent.setup();

    render(<WelcomeScreen />);

    const createButton = screen.getByRole('button', { name: '新規アカウント作成' });
    await user.click(createButton);

    // エラートーストが表示される
    await waitFor(() => {
      expect(toast.error).toHaveBeenCalledWith('アカウントの作成に失敗しました');
    });

    // errorHandlerが呼ばれる
    expect(errorHandler.log).toHaveBeenCalledWith('Failed to create account', error, {
      context: 'WelcomeScreen.handleCreateAccount',
    });

    // ナビゲーションは発生しない
    expect(mockNavigate).not.toHaveBeenCalled();
  });

  it('既存アカウントでログインボタンがクリックされた時、ログイン画面に遷移する', async () => {
    const user = userEvent.setup();

    render(<WelcomeScreen />);

    const loginButton = screen.getByRole('button', { name: '既存アカウントでログイン' });
    await user.click(loginButton);

    // ログイン画面への遷移（同期的に発生するはずなので、waitForは不要）
    expect(mockNavigate).toHaveBeenCalledWith({ to: '/login' });
  });

  it('ロゴが正しく表示される', () => {
    render(<WelcomeScreen />);

    // ロゴアイコン（Kの文字）
    expect(screen.getByText('K')).toBeInTheDocument();
    const logoContainer = screen.getByText('K').parentElement;
    expect(logoContainer).toHaveClass('bg-primary', 'rounded-full');
  });
});
