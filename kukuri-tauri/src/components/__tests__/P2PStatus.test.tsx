import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { P2PStatus } from '../P2PStatus';
import { useP2P, UseP2PReturn } from '@/hooks/useP2P';

// useP2Pフックのモック
vi.mock('@/hooks/useP2P');

// P2P APIのモック
vi.mock('@/lib/api/p2p', () => ({
  p2pApi: {
    initialize: vi.fn().mockResolvedValue(undefined),
    getNodeAddress: vi.fn().mockResolvedValue(['/ip4/127.0.0.1/tcp/4001']),
    getStatus: vi.fn().mockResolvedValue({
      connected: true,
      endpoint_id: 'test-node',
      active_topics: [],
      peer_count: 0,
    }),
    joinTopic: vi.fn().mockResolvedValue(undefined),
    leaveTopic: vi.fn().mockResolvedValue(undefined),
    broadcast: vi.fn().mockResolvedValue(undefined),
  },
}));

describe('P2PStatus', () => {
  const mockUseP2P: UseP2PReturn = {
    initialized: false,
    nodeId: null,
    nodeAddr: null,
    activeTopics: [],
    peers: [],
    connectionStatus: 'disconnected',
    error: null,
    joinTopic: vi.fn(),
    leaveTopic: vi.fn(),
    broadcast: vi.fn(),
    clearError: vi.fn(),
    getTopicMessages: vi.fn().mockReturnValue([]),
    getTopicStats: vi.fn(),
    isJoinedTopic: vi.fn().mockReturnValue(false),
    getConnectedPeerCount: vi.fn().mockReturnValue(0),
    getTopicPeerCount: vi.fn().mockReturnValue(0),
  };

  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useP2P).mockReturnValue(mockUseP2P);
  });

  describe('未接続状態', () => {
    it('未接続状態を正しく表示する', () => {
      render(<P2PStatus />);

      expect(screen.getByText('P2P ネットワーク')).toBeInTheDocument();
      expect(screen.getByText('分散型ネットワーク接続状態')).toBeInTheDocument();
      expect(screen.getByText('未接続')).toBeInTheDocument();
      expect(screen.getByText('P2Pネットワークに接続していません')).toBeInTheDocument();
    });
  });

  describe('接続中状態', () => {
    it('接続中状態を正しく表示する', () => {
      vi.mocked(useP2P).mockReturnValue({
        ...mockUseP2P,
        connectionStatus: 'connecting',
      });

      render(<P2PStatus />);

      expect(screen.getByText('接続中...')).toBeInTheDocument();
      expect(screen.getByText('ネットワークに接続中...')).toBeInTheDocument();
    });
  });

  describe('接続済み状態', () => {
    it('接続済み状態とノード情報を表示する', () => {
      vi.mocked(useP2P).mockReturnValue({
        ...mockUseP2P,
        initialized: true,
        connectionStatus: 'connected',
        nodeId: 'QmTestNodeId123456789',
        nodeAddr: '/ip4/127.0.0.1/tcp/4001/p2p/QmTestNodeId123456789',
        peers: [
          {
            node_id: 'peer1',
            node_addr: 'addr1',
            topics: [],
            last_seen: Date.now(),
            connection_status: 'connected',
          },
          {
            node_id: 'peer2',
            node_addr: 'addr2',
            topics: [],
            last_seen: Date.now(),
            connection_status: 'connected',
          },
          {
            node_id: 'peer3',
            node_addr: 'addr3',
            topics: [],
            last_seen: Date.now(),
            connection_status: 'disconnected',
          },
        ],
        activeTopics: [
          {
            topic_id: 'topic1',
            peer_count: 5,
            message_count: 100,
            recent_messages: [],
            connected_peers: [],
          },
          {
            topic_id: 'topic2',
            peer_count: 3,
            message_count: 50,
            recent_messages: [],
            connected_peers: [],
          },
        ],
        getConnectedPeerCount: vi.fn().mockReturnValue(2),
      });

      render(<P2PStatus />);

      expect(screen.getByText('接続中')).toBeInTheDocument();
      expect(screen.getByText('QmTestNodeId1234...')).toBeInTheDocument();
      expect(screen.getByText('2')).toBeInTheDocument(); // 接続ピア数
      expect(screen.getByText('2')).toBeInTheDocument(); // 参加トピック数
      expect(screen.getByText('topic1...')).toBeInTheDocument();
      expect(screen.getByText('topic2...')).toBeInTheDocument();
      expect(screen.getByText('100 msgs')).toBeInTheDocument();
      expect(screen.getByText('50 msgs')).toBeInTheDocument();
    });
  });

  describe('エラー状態', () => {
    it('エラー状態とメッセージを表示する', () => {
      vi.mocked(useP2P).mockReturnValue({
        ...mockUseP2P,
        connectionStatus: 'error',
        error: 'ネットワーク接続に失敗しました',
      });

      render(<P2PStatus />);

      expect(screen.getByText('エラー')).toBeInTheDocument();
      expect(screen.getByText('ネットワーク接続に失敗しました')).toBeInTheDocument();
      expect(screen.getByText('閉じる')).toBeInTheDocument();
    });

    it('エラーをクリアできる', () => {
      const clearError = vi.fn();
      vi.mocked(useP2P).mockReturnValue({
        ...mockUseP2P,
        connectionStatus: 'error',
        error: 'テストエラー',
        clearError,
      });

      render(<P2PStatus />);

      const closeButton = screen.getByText('閉じる');
      fireEvent.click(closeButton);

      expect(clearError).toHaveBeenCalledTimes(1);
    });
  });

  describe('アクティブトピック表示', () => {
    it('トピックがない場合の表示', () => {
      vi.mocked(useP2P).mockReturnValue({
        ...mockUseP2P,
        initialized: true,
        connectionStatus: 'connected',
        nodeId: 'QmTestNode',
        activeTopics: [],
      });

      render(<P2PStatus />);

      expect(screen.getByText('0')).toBeInTheDocument(); // 参加トピック数
    });

    it('複数のトピックを正しく表示する', () => {
      vi.mocked(useP2P).mockReturnValue({
        ...mockUseP2P,
        initialized: true,
        connectionStatus: 'connected',
        nodeId: 'QmTestNode',
        activeTopics: [
          {
            topic_id: 'topic-with-long-id-123456789',
            peer_count: 10,
            message_count: 250,
            recent_messages: [],
            connected_peers: [],
          },
        ],
      });

      render(<P2PStatus />);

      expect(screen.getByText('topic-wi...')).toBeInTheDocument();
      expect(screen.getByText('10')).toBeInTheDocument();
      expect(screen.getByText('250 msgs')).toBeInTheDocument();
    });
  });

  describe('ネットワークアドレス表示', () => {
    it('ネットワークアドレスが存在する場合に表示する', () => {
      vi.mocked(useP2P).mockReturnValue({
        ...mockUseP2P,
        initialized: true,
        connectionStatus: 'connected',
        nodeId: 'QmTestNode',
        nodeAddr: '/ip4/192.168.1.100/tcp/4001/p2p/QmTestNode',
      });

      render(<P2PStatus />);

      expect(screen.getByText('ネットワークアドレス')).toBeInTheDocument();
      expect(screen.getByText('/ip4/192.168.1.100/tcp/4001/p2p/QmTestNode')).toBeInTheDocument();
    });

    it('ネットワークアドレスがない場合は表示しない', () => {
      vi.mocked(useP2P).mockReturnValue({
        ...mockUseP2P,
        initialized: true,
        connectionStatus: 'connected',
        nodeId: 'QmTestNode',
        nodeAddr: null,
      });

      render(<P2PStatus />);

      expect(screen.queryByText('ネットワークアドレス')).not.toBeInTheDocument();
    });
  });
});
