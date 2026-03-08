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

    expect(screen.getByText('kukuri縺ｸ繧医≧縺薙◎')).toBeInTheDocument();
    expect(
      screen.getByText('蛻・淵蝙九ヨ繝斐ャ繧ｯ荳ｭ蠢・た繝ｼ繧ｷ繝｣繝ｫ繧｢繝励Μ繧ｱ繝ｼ繧ｷ繝ｧ繝ｳ'),
    ).toBeInTheDocument();
    expect(
      screen.getByText('繝ｻ繝医ヴ繝・け繝吶・繧ｹ縺ｮ繧ｿ繧､繝繝ｩ繧､繝ｳ縺ｧ諠・ｱ繧貞・譛・'),
    ).toBeInTheDocument();
    expect(screen.getByText('繝ｻP2P繝阪ャ繝医Ρ繝ｼ繧ｯ縺ｫ繧医ｋ讀憺夢閠先ｧ')).toBeInTheDocument();
    expect(
      screen.getByText('繝ｻNostr繝励Ο繝医さ繝ｫ縺ｫ繧医ｋ蛻・淵蝙九い繝ｼ繧ｭ繝・け繝√Ε'),
    ).toBeInTheDocument();
    expect(
      screen.getByRole('button', { name: '譁ｰ隕上い繧ｫ繧ｦ繝ｳ繝井ｽ懈・' }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole('button', { name: '譌｢蟄倥・骰ｵ縺ｧ繝ｭ繧ｰ繧､繝ｳ' }),
    ).toBeInTheDocument();
  });

  it('creates an account and navigates to profile setup', async () => {
    mockGenerateNewKeypair.mockResolvedValue({ nsec: 'nsec1test...' });
    const user = userEvent.setup();

    render(<WelcomeScreen />);

    const createButton = screen.getByRole('button', { name: '譁ｰ隕上い繧ｫ繧ｦ繝ｳ繝井ｽ懈・' });
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

    const createButton = screen.getByRole('button', { name: '譁ｰ隕上い繧ｫ繧ｦ繝ｳ繝井ｽ懈・' });
    const loginButton = screen.getByRole('button', { name: '譌｢蟄倥・骰ｵ縺ｧ繝ｭ繧ｰ繧､繝ｳ' });

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

    const createButton = screen.getByRole('button', { name: '譁ｰ隕上い繧ｫ繧ｦ繝ｳ繝井ｽ懈・' });
    await user.click(createButton);

    await waitFor(() => {
      expect(toast.error).toHaveBeenCalledWith('繧｢繧ｫ繧ｦ繝ｳ繝医・菴懈・縺ｫ螟ｱ謨励＠縺ｾ縺励◆');
    });

    expect(errorHandler.log).toHaveBeenCalledWith('Failed to create account', error, {
      context: 'WelcomeScreen.handleCreateAccount',
    });
    expect(mockNavigate).not.toHaveBeenCalled();
  });

  it('navigates to login', async () => {
    const user = userEvent.setup();

    render(<WelcomeScreen />);

    const loginButton = screen.getByRole('button', { name: '譌｢蟄倥・骰ｵ縺ｧ繝ｭ繧ｰ繧､繝ｳ' });
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
