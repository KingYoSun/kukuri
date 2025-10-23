import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { P2PStatus } from '@/components/P2PStatus';
import { useP2P, type UseP2PReturn } from '@/hooks/useP2P';

vi.mock('@/hooks/useP2P');

const baseReturn: UseP2PReturn = {
  initialized: false,
  nodeId: null,
  nodeAddr: null,
  activeTopics: [],
  peers: [],
  connectionStatus: 'disconnected',
  error: null,
  metricsSummary: null,
  joinTopic: vi.fn(),
  leaveTopic: vi.fn(),
  broadcast: vi.fn(),
  clearError: vi.fn(),
  refreshStatus: vi.fn(),
  getTopicMessages: vi.fn().mockReturnValue([]),
  getTopicStats: vi.fn(),
  isJoinedTopic: vi.fn().mockReturnValue(false),
  getConnectedPeerCount: vi.fn().mockReturnValue(0),
  getTopicPeerCount: vi.fn().mockReturnValue(0),
};

const mockUseP2PReturn = (override: Partial<UseP2PReturn> = {}): UseP2PReturn => ({
  ...baseReturn,
  ...override,
});

describe('P2PStatus', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    vi.mocked(useP2P).mockReturnValue(mockUseP2PReturn());
  });

  it('renders disconnected state by default', () => {
    render(<P2PStatus />);
    expect(screen.getByText('P2P ネットワーク')).toBeInTheDocument();
    expect(screen.getByText('分散型ネットワーク接続状態')).toBeInTheDocument();
    expect(screen.getByText('未接続')).toBeInTheDocument();
  });

  it('shows connecting badge when the state is connecting', () => {
    vi.mocked(useP2P).mockReturnValue(mockUseP2PReturn({ connectionStatus: 'connecting' }));
    render(<P2PStatus />);
    expect(screen.getByText('接続中...')).toBeInTheDocument();
  });

  it('renders connected view with metrics summary, peer counts and topics', () => {
    vi.mocked(useP2P).mockReturnValue(
      mockUseP2PReturn({
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
      }),
    );

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
    vi.mocked(useP2P).mockReturnValue(
      mockUseP2PReturn({
        initialized: true,
        connectionStatus: 'connected',
        refreshStatus,
        metricsSummary: {
          joins: 0,
          leaves: 0,
          broadcasts_sent: 0,
          messages_received: 0,
        },
      }),
    );

    render(<P2PStatus />);

    const callCountBefore = refreshStatus.mock.calls.length;
    fireEvent.click(screen.getByRole('button', { name: '更新' }));
    expect(refreshStatus.mock.calls.length).toBe(callCountBefore + 1);
  });

  it('renders error banner and clears it when 閉じる is pressed', () => {
    const clearError = vi.fn();
    vi.mocked(useP2P).mockReturnValue(
      mockUseP2PReturn({
        connectionStatus: 'error',
        error: '接続に失敗しました',
        clearError,
      }),
    );

    render(<P2PStatus />);
    expect(screen.getByText('エラー')).toBeInTheDocument();
    expect(screen.getByText('接続に失敗しました')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: '閉じる' }));
    expect(clearError).toHaveBeenCalledTimes(1);
  });
});
