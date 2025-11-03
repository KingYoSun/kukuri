import { render, screen, fireEvent, cleanup, act } from '@testing-library/react';
import { beforeEach, afterEach, describe, expect, it, vi } from 'vitest';
import { P2PStatus } from '@/components/P2PStatus';
import { useP2P, type UseP2PReturn } from '@/hooks/useP2P';

vi.mock('@/hooks/useP2P');

const mockedUseP2P = vi.mocked(useP2P);

const createBaseReturn = (): UseP2PReturn => ({
  initialized: false,
  nodeId: null,
  nodeAddr: null,
  activeTopics: [],
  peers: [],
  connectionStatus: 'disconnected',
  error: null,
  metricsSummary: null,
  statusError: null,
  statusBackoffMs: 30_000,
  lastStatusFetchedAt: Date.now(),
  isRefreshingStatus: false,
  joinTopic: vi.fn(),
  leaveTopic: vi.fn(),
  broadcast: vi.fn(),
  clearError: vi.fn(),
  refreshStatus: vi.fn().mockResolvedValue(undefined),
  getTopicMessages: vi.fn().mockReturnValue([]),
  getTopicStats: vi.fn(),
  isJoinedTopic: vi.fn().mockReturnValue(false),
  getConnectedPeerCount: vi.fn().mockReturnValue(0),
  getTopicPeerCount: vi.fn().mockReturnValue(0),
});

const setupUseP2P = (override: Partial<UseP2PReturn> = {}) => {
  const state = { ...createBaseReturn(), ...override };
  mockedUseP2P.mockReturnValue(state);
  return state;
};

describe('P2PStatus', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.clearAllMocks();
    setupUseP2P();
  });

  afterEach(() => {
    cleanup();
    vi.runOnlyPendingTimers();
    vi.useRealTimers();
    vi.clearAllMocks();
  });

  it('renders disconnected state by default', () => {
    render(<P2PStatus />);

    expect(screen.getByText('P2P ネットワーク')).toBeInTheDocument();
    expect(screen.getByText('分散型ネットワーク接続状態')).toBeInTheDocument();
    expect(screen.getByText('未接続')).toBeInTheDocument();
  });

  it('shows connecting badge when the state is connecting', () => {
    setupUseP2P({ connectionStatus: 'connecting' });

    render(<P2PStatus />);

    expect(screen.getByText('接続中...')).toBeInTheDocument();
  });

  it('renders connected view with metrics, peer counts and topics', () => {
    setupUseP2P({
      initialized: true,
      connectionStatus: 'connected',
      nodeId: 'QmExampleNodeId1234567890',
      nodeAddr: '/ip4/127.0.0.1/tcp/4001/p2p/QmExampleNodeId1234567890',
      peers: [
        {
          node_id: 'peer-1',
          node_addr: 'addr1',
          topics: [],
          last_seen: Date.now(),
          connection_status: 'connected',
        },
        {
          node_id: 'peer-2',
          node_addr: 'addr2',
          topics: [],
          last_seen: Date.now(),
          connection_status: 'connected',
        },
      ],
      activeTopics: [
        {
          topic_id: 'topic-1',
          peer_count: 3,
          message_count: 12,
          recent_messages: [],
          connected_peers: [],
        },
        {
          topic_id: 'topic-2',
          peer_count: 2,
          message_count: 5,
          recent_messages: [],
          connected_peers: [],
        },
      ],
      getConnectedPeerCount: vi.fn().mockReturnValue(2),
      metricsSummary: {
        joins: 4,
        leaves: 1,
        broadcasts_sent: 3,
        messages_received: 2,
      },
    });

    render(<P2PStatus />);

    expect(screen.getByText('接続中')).toBeInTheDocument();
    expect(screen.getByText('Gossipメトリクス')).toBeInTheDocument();
    expect(screen.getByText('Join').parentElement).toHaveTextContent('Join4');
    expect(screen.getByText('Leave').parentElement).toHaveTextContent('Leave1');
    expect(screen.getByText('Broadcast').parentElement).toHaveTextContent('Broadcast3');
    expect(screen.getByText('Received').parentElement).toHaveTextContent('Received2');
    expect(screen.getByText('topic-1...')).toBeInTheDocument();
    expect(screen.getByText('12 msgs')).toBeInTheDocument();
    expect(
      screen.getByText('/ip4/127.0.0.1/tcp/4001/p2p/QmExampleNodeId1234567890'),
    ).toBeInTheDocument();
  });

  it('calls refreshStatus when the 更新 button is pressed', () => {
    const refreshStatus = vi.fn().mockResolvedValue(undefined);

    setupUseP2P({
      initialized: true,
      connectionStatus: 'connected',
      refreshStatus,
      metricsSummary: {
        joins: 0,
        leaves: 0,
        broadcasts_sent: 0,
        messages_received: 0,
      },
    });

    render(<P2PStatus />);

    const before = refreshStatus.mock.calls.length;
    fireEvent.click(screen.getByRole('button', { name: '更新' }));
    expect(refreshStatus.mock.calls.length).toBe(before + 1);
  });

  it('renders error banner and clears it when 閉じる is pressed', () => {
    const clearError = vi.fn();

    setupUseP2P({
      connectionStatus: 'error',
      error: '接続に失敗しました',
      clearError,
      statusError: null,
    });

    render(<P2PStatus />);

    expect(screen.getByText('エラー')).toBeInTheDocument();
    expect(screen.getByText('接続に失敗しました')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: '閉じる' }));

    expect(clearError).toHaveBeenCalledTimes(1);
  });

  it('polls refreshStatus every 30 seconds when connected', () => {
    const refreshStatus = vi.fn().mockResolvedValue(undefined);

    setupUseP2P({
      initialized: true,
      connectionStatus: 'connected',
      refreshStatus,
    });

    render(<P2PStatus />);

    expect(refreshStatus).not.toHaveBeenCalled();

    act(() => {
      vi.advanceTimersByTime(30000);
    });

    expect(refreshStatus).toHaveBeenCalledTimes(1);
  });

  it('immediately refreshes when lastStatusFetchedAt is null', () => {
    const refreshStatus = vi.fn().mockResolvedValue(undefined);

    setupUseP2P({
      initialized: true,
      connectionStatus: 'connected',
      refreshStatus,
      lastStatusFetchedAt: null,
    });

    render(<P2PStatus />);

    expect(refreshStatus).toHaveBeenCalledTimes(1);
  });

  it('displays statusError message and allows retry', () => {
    const refreshStatus = vi.fn().mockResolvedValue(undefined);

    setupUseP2P({
      initialized: true,
      connectionStatus: 'connected',
      statusError: 'timeout',
      refreshStatus,
    });

    render(<P2PStatus />);

    expect(screen.getByText(/状態取得エラー/)).toBeInTheDocument();
    const retryButtons = screen.getAllByRole('button', { name: '再取得' });
    fireEvent.click(retryButtons[retryButtons.length - 1]);

    expect(refreshStatus).toHaveBeenCalledTimes(1);
  });

  it('disables manual refresh button when refreshing', () => {
    setupUseP2P({
      initialized: true,
      connectionStatus: 'connected',
      isRefreshingStatus: true,
    });

    render(<P2PStatus />);

    const loadingButton = screen.getByRole('button', { name: '更新中…' });
    expect(loadingButton).toBeDisabled();
  });

  it('requests refresh even when not connected to detect new peers', () => {
    const refreshStatus = vi.fn().mockResolvedValue(undefined);

    setupUseP2P({
      initialized: true,
      connectionStatus: 'disconnected',
      refreshStatus,
    });

    render(<P2PStatus />);

    expect(refreshStatus).not.toHaveBeenCalled();

    act(() => {
      vi.advanceTimersByTime(60000);
    });

    expect(refreshStatus).toHaveBeenCalledTimes(1);
  });

  it('clears scheduled refresh on unmount', () => {
    const refreshStatus = vi.fn().mockResolvedValue(undefined);
    const clearSpy = vi.spyOn(global, 'clearTimeout');

    setupUseP2P({
      initialized: true,
      connectionStatus: 'connected',
      refreshStatus,
    });

    const { unmount } = render(<P2PStatus />);

    unmount();

    expect(clearSpy).toHaveBeenCalled();

    clearSpy.mockRestore();
  });
});
