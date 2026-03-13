import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { WelcomeScreen } from '@/components/auth/WelcomeScreen';
import { useAuthStore } from '@/stores/authStore';
import { toast } from 'sonner';
import { errorHandler } from '@/lib/errorHandler';

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

  it('renders welcome content', () => {
    render(<WelcomeScreen />);

    expect(screen.getByText('kukuriへようこそ')).toBeInTheDocument();
    expect(screen.getByText('分散型トピック中心ソーシャルアプリケーション')).toBeInTheDocument();
    expect(screen.getByText('・トピックベースのタイムラインで情報を共有')).toBeInTheDocument();
    expect(screen.getByText('・P2Pネットワークによる検閲耐性')).toBeInTheDocument();
    expect(screen.getByText('・Nostrプロトコルによる分散型アーキテクチャ')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '新規アカウント作成' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '既存の鍵でログイン' })).toBeInTheDocument();
  });

  it('creates an account and navigates to profile setup', async () => {
    mockGenerateNewKeypair.mockResolvedValue({ nsec: 'nsec1test...' });
    const user = userEvent.setup();

    render(<WelcomeScreen />);

    const createButton = screen.getByRole('button', { name: '新規アカウント作成' });
    await user.click(createButton);

    await waitFor(() => {
      expect(mockGenerateNewKeypair).toHaveBeenCalledTimes(1);
      expect(mockGenerateNewKeypair).toHaveBeenCalledWith(true, { deferInitialization: true });
    });

    await waitFor(() => {
      expect(mockNavigate).toHaveBeenCalledWith({ to: '/profile-setup' });
    });
  });

  it('guards create-account against duplicate clicks', async () => {
    let resolveGeneration: ((value: { nsec: string }) => void) | null = null;
    mockGenerateNewKeypair.mockImplementation(
      () =>
        new Promise<{ nsec: string }>((resolve) => {
          resolveGeneration = resolve;
        }),
    );
    const user = userEvent.setup();

    render(<WelcomeScreen />);

    const createButton = screen.getByRole('button', { name: '新規アカウント作成' });
    const loginButton = screen.getByRole('button', { name: '既存の鍵でログイン' });

    await user.click(createButton);
    await user.click(createButton);

    expect(mockGenerateNewKeypair).toHaveBeenCalledTimes(1);
    expect(createButton).toBeDisabled();
    expect(loginButton).toBeDisabled();

    resolveGeneration?.({ nsec: 'nsec1test...' });

    await waitFor(() => {
      expect(mockNavigate).toHaveBeenCalledWith({ to: '/profile-setup' });
    });
  });

  it('shows an error toast when account creation fails', async () => {
    const error = new Error('Failed to generate keypair');
    mockGenerateNewKeypair.mockRejectedValue(error);
    const user = userEvent.setup();

    render(<WelcomeScreen />);

    const createButton = screen.getByRole('button', { name: '新規アカウント作成' });
    await user.click(createButton);

    await waitFor(() => {
      expect(toast.error).toHaveBeenCalledWith('アカウントの作成に失敗しました');
    });

    expect(errorHandler.log).toHaveBeenCalledWith('Failed to create account', error, {
      context: 'WelcomeScreen.handleCreateAccount',
    });
    expect(mockNavigate).not.toHaveBeenCalled();
  });

  it('navigates to login', async () => {
    const user = userEvent.setup();

    render(<WelcomeScreen />);

    const loginButton = screen.getByRole('button', { name: '既存の鍵でログイン' });
    await user.click(loginButton);

    expect(mockNavigate).toHaveBeenCalledWith({ to: '/login' });
  });

  it('renders the logo', () => {
    render(<WelcomeScreen />);

    expect(screen.getByText('K')).toBeInTheDocument();
    const logoContainer = screen.getByText('K').parentElement;
    expect(logoContainer).toHaveClass('bg-primary', 'rounded-full');
  });
});
