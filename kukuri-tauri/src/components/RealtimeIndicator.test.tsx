import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, act } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { RealtimeIndicator } from './RealtimeIndicator';

// navigatorのモック
Object.defineProperty(window, 'navigator', {
  value: {
    onLine: true,
  },
  writable: true,
});

describe('RealtimeIndicator', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    (window.navigator as Navigator & { onLine: boolean }).onLine = true;
  });

  it('should show online status when connected', () => {
    render(<RealtimeIndicator />);

    expect(screen.getByText('接続中')).toBeInTheDocument();
  });

  it('should show offline status when disconnected', () => {
    (window.navigator as Navigator & { onLine: boolean }).onLine = false;

    render(<RealtimeIndicator />);

    expect(screen.getByText('オフライン')).toBeInTheDocument();
  });

  it('should update status when connection changes', () => {
    const { rerender } = render(<RealtimeIndicator />);

    expect(screen.getByText('接続中')).toBeInTheDocument();

    // オフラインイベントを発火
    act(() => {
      (window.navigator as Navigator & { onLine: boolean }).onLine = false;
      window.dispatchEvent(new Event('offline'));
    });
    rerender(<RealtimeIndicator />);

    expect(screen.getByText('オフライン')).toBeInTheDocument();

    // オンラインイベントを発火
    act(() => {
      (window.navigator as Navigator & { onLine: boolean }).onLine = true;
      window.dispatchEvent(new Event('online'));
    });
    rerender(<RealtimeIndicator />);

    expect(screen.getByText('接続中')).toBeInTheDocument();
  });

  it('should show last update time in tooltip', async () => {
    const user = userEvent.setup();
    render(<RealtimeIndicator />);

    const indicator = screen.getByText('接続中').parentElement!;

    // ホバーしてツールチップを表示
    await user.hover(indicator);

    // ツールチップが表示されることを確認
    const tooltips = await screen.findAllByText(/リアルタイム更新:/);
    expect(tooltips.length).toBeGreaterThan(0);
    expect(tooltips[0]).toBeInTheDocument();
  });

  it('should update time display after realtime update event', () => {
    vi.useFakeTimers();
    const { rerender } = render(<RealtimeIndicator />);

    expect(screen.getByText('接続中')).toBeInTheDocument();

    // 15秒経過
    vi.advanceTimersByTime(15000);
    act(() => {
      window.dispatchEvent(new Event('realtime-update'));
    });
    rerender(<RealtimeIndicator />);

    // 再び接続中と表示される
    expect(screen.getByText('接続中')).toBeInTheDocument();

    vi.useRealTimers();
  });

  it('should show relative time correctly', () => {
    vi.useFakeTimers();
    const { rerender } = render(<RealtimeIndicator />);

    // 30秒経過
    vi.advanceTimersByTime(30000);
    rerender(<RealtimeIndicator />);
    expect(screen.getByText('30秒前')).toBeInTheDocument();

    // さらに30秒経過（合計1分）
    vi.advanceTimersByTime(30000);
    rerender(<RealtimeIndicator />);
    expect(screen.getByText('1分前')).toBeInTheDocument();

    // さらに59分経過（合計60分）
    vi.advanceTimersByTime(59 * 60 * 1000);
    rerender(<RealtimeIndicator />);
    expect(screen.getByText('1時間以上前')).toBeInTheDocument();

    vi.useRealTimers();
  });

  it('should apply correct styling for online state', () => {
    render(<RealtimeIndicator />);

    const indicator = screen.getByText('接続中').parentElement!;
    expect(indicator).toHaveClass('bg-green-100', 'text-green-800');
  });

  it('should apply correct styling for offline state', () => {
    (window.navigator as Navigator & { onLine: boolean }).onLine = false;
    render(<RealtimeIndicator />);

    const indicator = screen.getByText('オフライン').parentElement!;
    expect(indicator).toHaveClass('bg-red-100', 'text-red-800');
  });

  it('should cleanup event listeners on unmount', () => {
    const removeEventListenerSpy = vi.spyOn(window, 'removeEventListener');

    const { unmount } = render(<RealtimeIndicator />);
    unmount();

    expect(removeEventListenerSpy).toHaveBeenCalledWith('online', expect.any(Function));
    expect(removeEventListenerSpy).toHaveBeenCalledWith('offline', expect.any(Function));
    expect(removeEventListenerSpy).toHaveBeenCalledWith('realtime-update', expect.any(Function));
  });
});
