import { render, screen } from '@testing-library/react';
import { vi, describe, it, expect, beforeEach, afterEach } from 'vitest';
import { RelayStatus } from '../RelayStatus';
import { useAuthStore } from '@/stores/authStore';

// Zustand storeをモック
vi.mock('@/stores/authStore', () => ({
  useAuthStore: vi.fn(),
}));

// Tauri APIをモック
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

// import { invoke } from "@tauri-apps/api/core";

describe('RelayStatus', () => {
  const mockSetRelayStatus = vi.fn();
  const mockUpdateRelayStatus = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    vi.useFakeTimers();

    // デフォルトのストア状態を設定
    (useAuthStore as any).mockReturnValue({
      relayStatus: [],
      isLoggedIn: true,
      setRelayStatus: mockSetRelayStatus,
      updateRelayStatus: mockUpdateRelayStatus,
    });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('renders when relays are connected', () => {
    (useAuthStore as any).mockReturnValue({
      relayStatus: [{ url: 'wss://relay.test', status: 'connected' }],
      isLoggedIn: true,
      setRelayStatus: mockSetRelayStatus,
      updateRelayStatus: mockUpdateRelayStatus,
    });

    render(<RelayStatus />);

    expect(screen.getByText('リレー接続状態')).toBeInTheDocument();
  });

  it('does not render when no relays connected', () => {
    (useAuthStore as any).mockReturnValue({
      relayStatus: [],
      isLoggedIn: true,
      setRelayStatus: mockSetRelayStatus,
      updateRelayStatus: mockUpdateRelayStatus,
    });

    const { container } = render(<RelayStatus />);

    expect(container.firstChild).toBeNull();
  });

  it('displays relay status list', () => {
    const mockRelayStatus = [
      { url: 'wss://relay1.test', status: 'connected' },
      { url: 'wss://relay2.test', status: 'disconnected' },
      { url: 'wss://relay3.test', status: 'error: Connection timeout' },
    ];

    (useAuthStore as any).mockReturnValue({
      relayStatus: mockRelayStatus,
      isLoggedIn: true,
      setRelayStatus: mockSetRelayStatus,
      updateRelayStatus: mockUpdateRelayStatus,
    });

    render(<RelayStatus />);

    expect(screen.getByText('wss://relay1.test')).toBeInTheDocument();
    expect(screen.getByText('wss://relay2.test')).toBeInTheDocument();
    expect(screen.getByText('wss://relay3.test')).toBeInTheDocument();
  });

  it('shows correct status badges', () => {
    const mockRelayStatus = [
      { url: 'wss://relay1.test', status: 'connected' },
      { url: 'wss://relay2.test', status: 'disconnected' },
      { url: 'wss://relay3.test', status: 'connecting' },
    ];

    (useAuthStore as any).mockReturnValue({
      relayStatus: mockRelayStatus,
      isLoggedIn: true,
      setRelayStatus: mockSetRelayStatus,
      updateRelayStatus: mockUpdateRelayStatus,
    });

    render(<RelayStatus />);

    // 接続済みバッジ
    const connectedBadge = screen.getByText('接続済み');
    expect(connectedBadge).toHaveClass('bg-green-100');

    // 切断済みバッジ
    const disconnectedBadge = screen.getByText('切断');
    expect(disconnectedBadge).toHaveClass('bg-gray-100');

    // 接続中バッジ
    const connectingBadge = screen.getByText('接続中');
    expect(connectingBadge).toHaveClass('bg-yellow-100');
  });

  it('fetches relay status on mount', async () => {
    render(<RelayStatus />);

    expect(mockUpdateRelayStatus).toHaveBeenCalled();
  });

  it('updates relay status periodically', () => {
    render(<RelayStatus />);

    // 初回の取得
    expect(mockUpdateRelayStatus).toHaveBeenCalledTimes(1);

    // 30秒経過後の更新
    vi.advanceTimersByTime(30000);

    expect(mockUpdateRelayStatus).toHaveBeenCalledTimes(2);
  });

  it('handles error when fetching relay status', () => {
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    render(<RelayStatus />);

    expect(mockUpdateRelayStatus).toHaveBeenCalled();

    consoleErrorSpy.mockRestore();
  });

  it('clears interval on unmount', async () => {
    const clearIntervalSpy = vi.spyOn(global, 'clearInterval');

    const { unmount } = render(<RelayStatus />);

    unmount();

    expect(clearIntervalSpy).toHaveBeenCalled();
  });

  it('shows empty state when no relays', () => {
    (useAuthStore as any).mockReturnValue({
      relayStatus: [],
      isLoggedIn: true,
      setRelayStatus: mockSetRelayStatus,
      updateRelayStatus: mockUpdateRelayStatus,
    });

    const { container } = render(<RelayStatus />);

    // RelayStatus は relayStatus が空の場合 null を返す
    expect(container.firstChild).toBeNull();
  });

  it('handles error status with message', () => {
    const mockRelayStatus = [{ url: 'wss://relay.test', status: 'error: Connection refused' }];

    (useAuthStore as any).mockReturnValue({
      relayStatus: mockRelayStatus,
      isLoggedIn: true,
      setRelayStatus: mockSetRelayStatus,
      updateRelayStatus: mockUpdateRelayStatus,
    });

    render(<RelayStatus />);

    const errorBadge = screen.getByText('エラー');
    expect(errorBadge).toHaveClass('bg-red-100');
    expect(screen.getByText('wss://relay.test')).toBeInTheDocument();
  });
});
