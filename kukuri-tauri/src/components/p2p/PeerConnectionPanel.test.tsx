import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { PeerConnectionPanel } from './PeerConnectionPanel';
import { useP2PStore } from '@/stores/p2pStore';
import { p2pApi } from '@/lib/api/p2p';

// モック
vi.mock('@/stores/p2pStore');
vi.mock('@/lib/api/p2p');

const mockUseP2PStore = vi.mocked(useP2PStore);
const mockP2pApi = vi.mocked(p2pApi);

// useToastのモック
const mockToast = vi.fn();
vi.mock('@/hooks/use-toast', () => ({
  useToast: () => ({
    toast: mockToast,
  }),
}));

describe('PeerConnectionPanel', () => {
  const mockInitialize = vi.fn();

  beforeEach(() => {
    // localStorage のモック
    const localStorageMock = {
      getItem: vi.fn(),
      setItem: vi.fn(),
      removeItem: vi.fn(),
      clear: vi.fn(),
    };
    Object.defineProperty(window, 'localStorage', {
      value: localStorageMock,
      writable: true,
    });

    // クリップボードAPIのモック
    Object.defineProperty(navigator, 'clipboard', {
      value: {
        writeText: vi.fn().mockResolvedValue(undefined),
      },
      writable: true,
      configurable: true,
    });

    // useP2PStore のモック
    mockUseP2PStore.mockReturnValue({
      nodeAddr: '/ip4/192.168.1.100/tcp/4001/p2p/12D3KooWExample',
      connectionStatus: 'connected',
      initialize: mockInitialize,
    } as Partial<ReturnType<typeof useP2P>>);

    // mockToastをクリア
    mockToast.mockClear();

    // p2pApi のモック
    mockP2pApi.connectToPeer = vi.fn().mockResolvedValue(undefined);
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it('自分のピアアドレスを表示する', () => {
    render(<PeerConnectionPanel />);

    const addressInput = screen.getByDisplayValue(
      '/ip4/192.168.1.100/tcp/4001/p2p/12D3KooWExample',
    );
    expect(addressInput).toBeInTheDocument();
    expect(addressInput).toHaveAttribute('readOnly');
  });

  it('接続状態がdisconnectedの場合、初期化を呼び出す', () => {
    mockUseP2PStore.mockReturnValue({
      nodeAddr: null,
      connectionStatus: 'disconnected',
      initialize: mockInitialize,
    } as Partial<ReturnType<typeof useP2P>>);

    render(<PeerConnectionPanel />);

    expect(mockInitialize).toHaveBeenCalled();
  });

  it('コピーボタンをクリックするとアドレスをクリップボードにコピーする', async () => {
    render(<PeerConnectionPanel />);

    const copyButton = screen.getByTitle('コピー');
    await userEvent.click(copyButton);

    expect(navigator.clipboard.writeText).toHaveBeenCalledWith(
      '/ip4/192.168.1.100/tcp/4001/p2p/12D3KooWExample',
    );
    expect(mockToast).toHaveBeenCalledWith({
      title: 'コピーしました',
      description: 'ピアアドレスをクリップボードにコピーしました',
    });
  });

  it('有効なピアアドレスで接続を実行する', async () => {
    const user = userEvent.setup();
    render(<PeerConnectionPanel />);

    const input = screen.getByPlaceholderText('/ip4/192.168.1.100/tcp/4001/p2p/QmXXX...');
    const connectButton = screen.getByText('接続');

    await user.type(input, '/ip4/10.0.0.1/tcp/4001/p2p/12D3KooWTest');
    await user.click(connectButton);

    await waitFor(() => {
      expect(mockP2pApi.connectToPeer).toHaveBeenCalledWith(
        '/ip4/10.0.0.1/tcp/4001/p2p/12D3KooWTest',
      );
      expect(mockToast).toHaveBeenCalledWith({
        title: '接続成功',
        description: 'ピアに接続しました',
      });
    });
  });

  it.skip('無効なピアアドレスでエラーを表示する', async () => {
    const user = userEvent.setup();
    render(<PeerConnectionPanel />);

    const input = screen.getByPlaceholderText('/ip4/192.168.1.100/tcp/4001/p2p/QmXXX...');
    const connectButton = screen.getByText('接続');

    await user.type(input, 'invalid-address');
    fireEvent.click(connectButton);

    expect(mockToast).toHaveBeenCalledWith({
      title: 'エラー',
      description: '無効なピアアドレス形式です',
      variant: 'destructive',
    });
    expect(mockP2pApi.connectToPeer).not.toHaveBeenCalled();
  });

  it.skip('空のアドレスでエラーを表示する', async () => {
    render(<PeerConnectionPanel />);

    const connectButton = screen.getByText('接続');
    fireEvent.click(connectButton);

    expect(mockToast).toHaveBeenCalledWith({
      title: 'エラー',
      description: 'ピアアドレスを入力してください',
      variant: 'destructive',
    });
    expect(mockP2pApi.connectToPeer).not.toHaveBeenCalled();
  });

  it.skip('接続失敗時にエラーを表示する', async () => {
    const user = userEvent.setup();
    mockP2pApi.connectToPeer = vi.fn().mockRejectedValue(new Error('Connection failed'));

    render(<PeerConnectionPanel />);

    const input = screen.getByPlaceholderText('/ip4/192.168.1.100/tcp/4001/p2p/QmXXX...');
    const connectButton = screen.getByText('接続');

    await user.type(input, '/ip4/10.0.0.1/tcp/4001/p2p/12D3KooWTest');
    await user.click(connectButton);

    await waitFor(() => {
      expect(mockP2pApi.connectToPeer).toHaveBeenCalled();
      // errorHandler経由でtoastが呼ばれることを確認
      expect(window.localStorage.setItem).toHaveBeenCalled();
    });
  });

  it('接続履歴を表示する', () => {
    const mockHistory = [
      {
        id: '1',
        address: '/ip4/10.0.0.1/tcp/4001/p2p/12D3KooWTest1',
        timestamp: Date.now() - 3600000,
        status: 'connected' as const,
      },
      {
        id: '2',
        address: '/ip4/10.0.0.2/tcp/4001/p2p/12D3KooWTest2',
        timestamp: Date.now() - 7200000,
        status: 'failed' as const,
      },
    ];

    window.localStorage.getItem = vi.fn().mockReturnValue(JSON.stringify(mockHistory));

    render(<PeerConnectionPanel />);

    expect(screen.getByText('接続履歴')).toBeInTheDocument();
    expect(screen.getByText('/ip4/10.0.0.1/tcp/4001/p2p/12D3KooWTest1')).toBeInTheDocument();
    expect(screen.getByText('/ip4/10.0.0.2/tcp/4001/p2p/12D3KooWTest2')).toBeInTheDocument();
    expect(screen.getByText('接続失敗')).toBeInTheDocument();
  });

  it.skip('接続履歴から再接続する', async () => {
    const user = userEvent.setup();
    const mockHistory = [
      {
        id: '1',
        address: '/ip4/10.0.0.1/tcp/4001/p2p/12D3KooWTest1',
        timestamp: Date.now() - 3600000,
        status: 'connected' as const,
      },
    ];

    window.localStorage.getItem = vi.fn().mockReturnValue(JSON.stringify(mockHistory));

    render(<PeerConnectionPanel />);

    const reconnectButton = screen.getByText('再接続');
    await user.click(reconnectButton);

    await waitFor(() => {
      expect(mockP2pApi.connectToPeer).toHaveBeenCalledWith(
        '/ip4/10.0.0.1/tcp/4001/p2p/12D3KooWTest1',
      );
    });
  });

  it('接続履歴をクリアする', async () => {
    const user = userEvent.setup();
    const mockHistory = [
      {
        id: '1',
        address: '/ip4/10.0.0.1/tcp/4001/p2p/12D3KooWTest1',
        timestamp: Date.now() - 3600000,
        status: 'connected' as const,
      },
    ];

    window.localStorage.getItem = vi.fn().mockReturnValue(JSON.stringify(mockHistory));

    render(<PeerConnectionPanel />);

    const clearButton = screen.getByText('履歴をクリア');
    await user.click(clearButton);

    expect(window.localStorage.removeItem).toHaveBeenCalledWith('p2p-connection-history');
    expect(mockToast).toHaveBeenCalledWith({
      title: '履歴をクリアしました',
      description: '接続履歴を削除しました',
    });
  });

  it('Enterキーで接続を実行する', async () => {
    const user = userEvent.setup();
    render(<PeerConnectionPanel />);

    const input = screen.getByPlaceholderText('/ip4/192.168.1.100/tcp/4001/p2p/QmXXX...');

    await user.type(input, '/ip4/10.0.0.1/tcp/4001/p2p/12D3KooWTest');
    await user.keyboard('{Enter}');

    await waitFor(() => {
      expect(mockP2pApi.connectToPeer).toHaveBeenCalledWith(
        '/ip4/10.0.0.1/tcp/4001/p2p/12D3KooWTest',
      );
    });
  });

  it('IPv6アドレスでの接続を許可する', async () => {
    const user = userEvent.setup();
    render(<PeerConnectionPanel />);

    const input = screen.getByPlaceholderText('/ip4/192.168.1.100/tcp/4001/p2p/QmXXX...');
    const connectButton = screen.getByText('接続');

    await user.type(input, '/ip6/2001:db8::1/tcp/4001/p2p/12D3KooWTest');
    await user.click(connectButton);

    await waitFor(() => {
      expect(mockP2pApi.connectToPeer).toHaveBeenCalledWith(
        '/ip6/2001:db8::1/tcp/4001/p2p/12D3KooWTest',
      );
      expect(mockToast).toHaveBeenCalledWith({
        title: '接続成功',
        description: 'ピアに接続しました',
      });
    });
  });

  it('nodeAddrがnullの場合、ローディング状態を表示する', () => {
    mockUseP2PStore.mockReturnValue({
      nodeAddr: null,
      connectionStatus: 'connected',
      initialize: mockInitialize,
    } as Partial<ReturnType<typeof useP2P>>);

    render(<PeerConnectionPanel />);

    expect(screen.getByText('アドレスを取得中...')).toBeInTheDocument();
  });
});
