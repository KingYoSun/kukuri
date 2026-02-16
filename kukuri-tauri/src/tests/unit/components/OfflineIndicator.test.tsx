import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, act } from '@testing-library/react';
import { OfflineIndicator } from '@/components/OfflineIndicator';
import { useOfflineStore } from '@/stores/offlineStore';
import { formatDistanceToNow } from 'date-fns';

vi.mock('@/stores/offlineStore');
vi.mock('date-fns', () => ({
  formatDistanceToNow: vi.fn(),
}));

describe('OfflineIndicator', () => {
  const mockUseOfflineStore = useOfflineStore as unknown as ReturnType<typeof vi.fn>;
  const mockFormatDistanceToNow = formatDistanceToNow as unknown as ReturnType<typeof vi.fn>;

  beforeEach(() => {
    vi.clearAllMocks();
    mockFormatDistanceToNow.mockReturnValue('5分前');
  });

  it('オンラインで未同期アクションがない場合は表示されない', () => {
    mockUseOfflineStore.mockReturnValue({
      isOnline: true,
      lastSyncedAt: Date.now(),
      pendingActions: [],
      isSyncing: false,
    });

    const { container } = render(<OfflineIndicator />);
    expect(container.firstChild).toBeNull();
  });

  it('オフライン時にバナーが表示される', () => {
    mockUseOfflineStore.mockReturnValue({
      isOnline: false,
      lastSyncedAt: Date.now(),
      pendingActions: [],
      isSyncing: false,
    });

    render(<OfflineIndicator />);
    expect(screen.getByText('オフラインモード')).toBeInTheDocument();
    expect(screen.getByText('変更は保存され、オンライン時に同期されます')).toBeInTheDocument();
  });

  it('オンライン復帰時に成功メッセージが表示される', async () => {
    const { rerender } = render(<OfflineIndicator />);

    mockUseOfflineStore.mockReturnValue({
      isOnline: false,
      lastSyncedAt: Date.now(),
      pendingActions: [],
      isSyncing: false,
    });
    await act(async () => {
      rerender(<OfflineIndicator />);
    });

    mockUseOfflineStore.mockReturnValue({
      isOnline: true,
      lastSyncedAt: Date.now(),
      pendingActions: [],
      isSyncing: false,
    });
    await act(async () => {
      rerender(<OfflineIndicator />);
    });

    await waitFor(() => {
      expect(screen.getByText('オンラインに復帰しました')).toBeInTheDocument();
    });
  });

  it('同期中の状態が表示される', () => {
    mockUseOfflineStore.mockReturnValue({
      isOnline: false,
      lastSyncedAt: Date.now(),
      pendingActions: [],
      isSyncing: true,
    });

    render(<OfflineIndicator />);
    expect(screen.getByText('オフラインモード')).toBeInTheDocument();
  });

  it('未同期アクションの数が表示される', () => {
    mockUseOfflineStore.mockReturnValue({
      isOnline: true,
      lastSyncedAt: Date.now(),
      pendingActions: [
        { localId: '1', action: {}, createdAt: Date.now() },
        { localId: '2', action: {}, createdAt: Date.now() },
        { localId: '3', action: {}, createdAt: Date.now() },
      ],
      isSyncing: false,
    });

    render(<OfflineIndicator />);

    const trigger = screen.getByRole('button');
    act(() => {
      trigger.focus();
    });

    const messages = screen.getAllByText('未同期アクション: 3件');
    expect(messages.length).toBeGreaterThan(0);
  });

  it('最終同期時刻が表示される', () => {
    const lastSyncedAt = Date.now() - 300000; // 5分前
    mockUseOfflineStore.mockReturnValue({
      isOnline: true,
      lastSyncedAt,
      pendingActions: [{ localId: '1', action: {}, createdAt: lastSyncedAt }],
      isSyncing: false,
    });

    render(<OfflineIndicator />);
    expect(mockFormatDistanceToNow).toHaveBeenCalledWith(
      lastSyncedAt,
      expect.objectContaining({
        addSuffix: true,
        locale: expect.objectContaining({
          code: 'ja',
        }),
      }),
    );
  });

  it('同期履歴がない場合は「未同期」と表示される', async () => {
    mockUseOfflineStore.mockReturnValue({
      isOnline: false,
      lastSyncedAt: undefined,
      pendingActions: [],
      isSyncing: false,
    });

    await act(async () => {
      render(<OfflineIndicator />);
    });

    const tooltipTrigger = screen.getByRole('button');
    await act(async () => {
      tooltipTrigger.focus();
    });

    await waitFor(() => {
      expect(mockFormatDistanceToNow).not.toHaveBeenCalled();
    });
  });

  it('SyncStatusIndicator への導線メッセージを表示する', async () => {
    mockUseOfflineStore.mockReturnValue({
      isOnline: false,
      lastSyncedAt: Date.now(),
      pendingActions: [{ localId: '1', action: {}, createdAt: Date.now() }],
      isSyncing: false,
    });

    await act(async () => {
      render(<OfflineIndicator />);
    });

    const trigger = screen.getByRole('button');
    await act(async () => {
      trigger.focus();
    });

    await waitFor(() => {
      const guidance = screen.getAllByText(
        '詳細なステータスはヘッダー右上の SyncStatusIndicator から確認できます',
      );
      expect(guidance.length).toBeGreaterThan(0);
    });
  });

  it('オフライン時に未同期アクション数が案内される', async () => {
    mockUseOfflineStore.mockReturnValue({
      isOnline: false,
      lastSyncedAt: Date.now(),
      pendingActions: [
        { localId: '1', action: {}, createdAt: Date.now() },
        { localId: '2', action: {}, createdAt: Date.now() },
      ],
      isSyncing: false,
    });

    render(<OfflineIndicator />);

    const trigger = screen.getByRole('button');
    await act(async () => {
      trigger.focus();
    });

    expect(screen.getAllByText('未同期アクション: 2件').length).toBeGreaterThan(0);
  });

  it.skip('オンライン復帰後5秒でバナーが自動的に非表示になる', async () => {
    vi.useFakeTimers();

    mockUseOfflineStore.mockReturnValue({
      isOnline: false,
      lastSyncedAt: Date.now(),
      pendingActions: [],
      isSyncing: false,
    });
    const { rerender } = render(<OfflineIndicator />);

    mockUseOfflineStore.mockReturnValue({
      isOnline: true,
      lastSyncedAt: Date.now(),
      pendingActions: [],
      isSyncing: false,
    });
    await act(async () => {
      rerender(<OfflineIndicator />);
    });

    await waitFor(() => {
      expect(screen.getByText('オンラインに復帰しました')).toBeInTheDocument();
    });

    await act(async () => {
      await vi.advanceTimersByTimeAsync(5000);
    });

    await waitFor(() => {
      expect(screen.queryByText('オンラインに復帰しました')).not.toBeInTheDocument();
    });

    vi.useRealTimers();
  }, 10000);
});
