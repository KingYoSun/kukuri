import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { Route } from '../__root';
import { useAuthStore } from '@/stores/authStore';
import { useTopics, useP2P } from '@/hooks';

// モック
const mockNavigate = vi.fn();
vi.mock('@tanstack/react-router', () => ({
  createRootRoute: vi.fn((config: any) => ({ component: config.component })),
  Outlet: () => <div data-testid="outlet">Outlet</div>,
  useNavigate: () => mockNavigate,
}));

vi.mock('@/components/layout/MainLayout', () => ({
  MainLayout: ({ children }: { children: React.ReactNode }) => (
    <div data-testid="main-layout">{children}</div>
  ),
}));

vi.mock('@/stores/authStore');
vi.mock('@/hooks');

// window.location.pathnameのモック
const mockPathname = vi.fn();
Object.defineProperty(window, 'location', {
  value: {
    get pathname() {
      return mockPathname();
    },
  },
  configurable: true,
});

describe('__root (Authentication Guard)', () => {
  const mockInitialize = vi.fn();
  const RootComponent = Route.component;
  
  beforeEach(() => {
    vi.clearAllMocks();
    mockPathname.mockReturnValue('/');
    
    // デフォルトのモック設定
    (useAuthStore as unknown as vi.Mock).mockReturnValue({
      isAuthenticated: false,
      initialize: mockInitialize,
    });
    
    (useTopics as vi.Mock).mockReturnValue({
      data: [],
      isLoading: false,
    });
    
    (useP2P as vi.Mock).mockReturnValue({
      initialized: false,
    });
    
    (useAuth as vi.Mock).mockReturnValue({});
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('初期化中は初期化メッセージを表示する', async () => {
    mockInitialize.mockImplementation(() => new Promise(() => {})); // 永続的にpending
    
    render(<RootComponent />);
    
    expect(screen.getByText('初期化中...')).toBeInTheDocument();
    
    await waitFor(() => {
      expect(mockInitialize).toHaveBeenCalledTimes(1);
    });
  });

  it('初期化完了後、未認証で保護されたページにアクセスするとウェルカム画面にリダイレクトする', async () => {
    mockInitialize.mockResolvedValue(undefined);
    mockPathname.mockReturnValue('/');
    
    (useAuthStore as unknown as vi.Mock).mockReturnValue({
      isAuthenticated: false,
      initialize: mockInitialize,
    });
    
    render(<RootComponent />);
    
    // 初期化完了を待つ
    await waitFor(() => {
      expect(mockNavigate).toHaveBeenCalledWith({ to: '/welcome' });
    });
  });

  it('初期化完了後、認証済みで認証ページにアクセスするとホーム画面にリダイレクトする', async () => {
    mockInitialize.mockResolvedValue(undefined);
    mockPathname.mockReturnValue('/welcome');
    
    (useAuthStore as unknown as vi.Mock).mockReturnValue({
      isAuthenticated: true,
      initialize: mockInitialize,
    });
    
    render(<RootComponent />);
    
    // 初期化完了を待つ
    await waitFor(() => {
      expect(mockNavigate).toHaveBeenCalledWith({ to: '/' });
    });
  });

  it('未認証で保護されたページにアクセスするとリダイレクト中メッセージを表示する', async () => {
    mockInitialize.mockResolvedValue(undefined);
    mockPathname.mockReturnValue('/topics');
    
    (useAuthStore as unknown as vi.Mock).mockReturnValue({
      isAuthenticated: false,
      initialize: mockInitialize,
    });
    
    render(<RootComponent />);
    
    await waitFor(() => {
      expect(screen.getByText('リダイレクト中...')).toBeInTheDocument();
    });
  });


  it('認証済みで通常ページにアクセスするとレイアウトありで表示される', async () => {
    mockInitialize.mockResolvedValue(undefined);
    mockPathname.mockReturnValue('/');
    
    (useAuthStore as unknown as vi.Mock).mockReturnValue({
      isAuthenticated: true,
      initialize: mockInitialize,
    });
    
    render(<RootComponent />);
    
    await waitFor(() => {
      expect(screen.getByTestId('main-layout')).toBeInTheDocument();
      expect(screen.getByTestId('outlet')).toBeInTheDocument();
    });
  });

  it('トピックデータ読み込み中は読み込み中メッセージを表示する', async () => {
    mockInitialize.mockResolvedValue(undefined);
    mockPathname.mockReturnValue('/');
    
    (useAuthStore as unknown as vi.Mock).mockReturnValue({
      isAuthenticated: true,
      initialize: mockInitialize,
    });
    
    (useTopics as vi.Mock).mockReturnValue({
      data: null,
      isLoading: true,
    });
    
    render(<RootComponent />);
    
    await waitFor(() => {
      expect(screen.getByText('読み込み中...')).toBeInTheDocument();
    });
  });

  it('保護されたパスのリストが正しく設定されている', async () => {
    mockInitialize.mockResolvedValue(undefined);
    
    const protectedPaths = ['/', '/topics', '/settings'];
    
    for (const path of protectedPaths) {
      vi.clearAllMocks();
      mockPathname.mockReturnValue(path);
      
      (useAuthStore as unknown as vi.Mock).mockReturnValue({
        isAuthenticated: false,
        initialize: mockInitialize,
      });
      
      render(<RootComponent />);
      
      await waitFor(() => {
        expect(mockNavigate).toHaveBeenCalledWith({ to: '/welcome' });
      });
    }
  });

  it('認証ページのリストが正しく設定されている', async () => {
    mockInitialize.mockResolvedValue(undefined);
    
    const authPaths = ['/welcome', '/login', '/profile-setup'];
    
    for (const path of authPaths) {
      vi.clearAllMocks();
      mockPathname.mockReturnValue(path);
      
      (useAuthStore as unknown as vi.Mock).mockReturnValue({
        isAuthenticated: true,
        initialize: mockInitialize,
      });
      
      render(<RootComponent />);
      
      await waitFor(() => {
        expect(mockNavigate).toHaveBeenCalledWith({ to: '/' });
      });
    }
  });

  it('トピックデータが読み込まれた時にコンソールログが出力される', async () => {
    const consoleLogSpy = vi.spyOn(console, 'log').mockImplementation(() => {});
    mockInitialize.mockResolvedValue(undefined);
    mockPathname.mockReturnValue('/');
    
    const mockTopics = [{ id: '1', name: 'Test Topic' }];
    
    (useAuthStore as unknown as vi.Mock).mockReturnValue({
      isAuthenticated: true,
      initialize: mockInitialize,
    });
    
    (useTopics as vi.Mock).mockReturnValue({
      data: mockTopics,
      isLoading: false,
    });
    
    render(<RootComponent />);
    
    await waitFor(() => {
      expect(consoleLogSpy).toHaveBeenCalledWith('Topics loaded:', mockTopics);
    });
    
    consoleLogSpy.mockRestore();
  });

  it('P2P初期化状態がコンソールログに出力される', async () => {
    const consoleLogSpy = vi.spyOn(console, 'log').mockImplementation(() => {});
    mockInitialize.mockResolvedValue(undefined);
    mockPathname.mockReturnValue('/');
    
    (useAuthStore as unknown as vi.Mock).mockReturnValue({
      isAuthenticated: true,
      initialize: mockInitialize,
    });
    
    (useP2P as vi.Mock).mockReturnValue({
      initialized: true,
    });
    
    render(<RootComponent />);
    
    await waitFor(() => {
      expect(consoleLogSpy).toHaveBeenCalledWith('P2P initialized:', true);
    });
    
    consoleLogSpy.mockRestore();
  });
});