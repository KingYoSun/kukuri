import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { P2PDebugPanel } from '../P2PDebugPanel';
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

describe('P2PDebugPanel', () => {
  const mockUseP2P: UseP2PReturn = {
    initialized: true,
    nodeId: 'QmTestNode123',
    nodeAddr: '/ip4/127.0.0.1/tcp/4001/p2p/QmTestNode123',
    activeTopics: [],
    peers: [],
    connectionStatus: 'connected',
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

  describe('基本的な表示', () => {
    it('P2Pデバッグパネルが表示される', () => {
      render(<P2PDebugPanel />);
      expect(screen.getByText('P2P Debug Panel')).toBeInTheDocument();
    });
  });

  describe('状態タブ', () => {
    it('接続状態が正しく表示される', () => {
      render(<P2PDebugPanel />);

      expect(screen.getByText('接続状態')).toBeInTheDocument();
      expect(screen.getByText('connected')).toBeInTheDocument();
      expect(screen.getByText('QmTestNode123')).toBeInTheDocument();
      expect(screen.getByText('/ip4/127.0.0.1/tcp/4001/p2p/QmTestNode123')).toBeInTheDocument();
    });

    it('ピア数とトピック数が表示される', () => {
      vi.mocked(useP2P).mockReturnValue({
        ...mockUseP2P,
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
        ],
        activeTopics: [
          {
            topic_id: 'topic1',
            peer_count: 1,
            message_count: 0,
            recent_messages: [],
            connected_peers: [],
          },
          {
            topic_id: 'topic2',
            peer_count: 1,
            message_count: 0,
            recent_messages: [],
            connected_peers: [],
          },
          {
            topic_id: 'topic3',
            peer_count: 1,
            message_count: 0,
            recent_messages: [],
            connected_peers: [],
          },
        ],
      });

      render(<P2PDebugPanel />);

      expect(screen.getByText('2')).toBeInTheDocument(); // ピア数
      expect(screen.getByText('3')).toBeInTheDocument(); // トピック数
    });

    it('エラーが表示され、クリアできる', () => {
      vi.mocked(useP2P).mockReturnValue({
        ...mockUseP2P,
        error: 'テストエラーメッセージ',
      });

      render(<P2PDebugPanel />);

      expect(screen.getByText('テストエラーメッセージ')).toBeInTheDocument();

      const clearButton = screen.getByText('エラーをクリア');
      fireEvent.click(clearButton);

      expect(mockUseP2P.clearError).toHaveBeenCalledTimes(1);
    });
  });

  describe('トピックタブ', () => {
    it('新しいトピックに参加できる', async () => {
      render(<P2PDebugPanel />);

      // トピックタブに切り替え
      const topicsTab = screen.getByText('トピック');
      fireEvent.click(topicsTab);

      const input = screen.getByPlaceholderText('トピックID (例: test-topic)');
      fireEvent.change(input, { target: { value: 'new-topic' } });

      const joinButton = screen.getByText('参加');
      fireEvent.click(joinButton);

      await waitFor(() => {
        expect(mockUseP2P.joinTopic).toHaveBeenCalledWith('new-topic');
      });
    });

    it('参加中のトピック一覧が表示される', () => {
      vi.mocked(useP2P).mockReturnValue({
        ...mockUseP2P,
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
      });

      render(<P2PDebugPanel />);

      const topicsTab = screen.getByText('トピック');
      fireEvent.click(topicsTab);

      expect(screen.getByText('topic1')).toBeInTheDocument();
      expect(screen.getByText('ピア: 5')).toBeInTheDocument();
      expect(screen.getByText('メッセージ: 100')).toBeInTheDocument();

      expect(screen.getByText('topic2')).toBeInTheDocument();
      expect(screen.getByText('ピア: 3')).toBeInTheDocument();
      expect(screen.getByText('メッセージ: 50')).toBeInTheDocument();
    });

    it('トピックから離脱できる', async () => {
      vi.mocked(useP2P).mockReturnValue({
        ...mockUseP2P,
        activeTopics: [
          {
            topic_id: 'topic1',
            peer_count: 1,
            message_count: 0,
            recent_messages: [],
            connected_peers: [],
          },
        ],
      });

      render(<P2PDebugPanel />);

      const topicsTab = screen.getByText('トピック');
      fireEvent.click(topicsTab);

      // 削除ボタンをクリック（TrashIconを含むボタン）
      const deleteButtons = screen
        .getAllByRole('button')
        .filter((button) => button.querySelector('svg'));
      const deleteButton = deleteButtons[deleteButtons.length - 1];
      fireEvent.click(deleteButton);

      await waitFor(() => {
        expect(mockUseP2P.leaveTopic).toHaveBeenCalledWith('topic1');
      });
    });
  });

  describe('送信タブ', () => {
    it('トピックが選択されていない場合のメッセージ', () => {
      render(<P2PDebugPanel />);

      const broadcastTab = screen.getByText('送信');
      fireEvent.click(broadcastTab);

      expect(screen.getByText('トピックを選択してください')).toBeInTheDocument();
    });

    it('メッセージをブロードキャストできる', async () => {
      vi.mocked(useP2P).mockReturnValue({
        ...mockUseP2P,
        activeTopics: [
          {
            topic_id: 'topic1',
            peer_count: 2,
            message_count: 10,
            recent_messages: [],
            connected_peers: [],
          },
        ],
      });

      render(<P2PDebugPanel />);

      // トピックタブでトピックを選択
      const topicsTab = screen.getByText('トピック');
      fireEvent.click(topicsTab);

      const selectButton = screen.getByText('選択');
      fireEvent.click(selectButton);

      // 送信タブに切り替え
      const broadcastTab = screen.getByText('送信');
      fireEvent.click(broadcastTab);

      expect(screen.getByText('topic1')).toBeInTheDocument();

      const messageInput = screen.getByPlaceholderText('送信するメッセージを入力');
      fireEvent.change(messageInput, { target: { value: 'Hello P2P!' } });

      const sendButton = screen.getByText('ブロードキャスト');
      fireEvent.click(sendButton);

      await waitFor(() => {
        expect(mockUseP2P.broadcast).toHaveBeenCalledWith('topic1', 'Hello P2P!');
      });
    });
  });

  describe('ログタブ', () => {
    it('初期状態ではログが空', () => {
      render(<P2PDebugPanel />);

      const logsTab = screen.getByText('ログ');
      fireEvent.click(logsTab);

      expect(screen.getByText('ログはありません')).toBeInTheDocument();
    });

    it('操作ログが記録される', async () => {
      render(<P2PDebugPanel />);

      // トピックに参加
      const topicsTab = screen.getByText('トピック');
      fireEvent.click(topicsTab);

      const input = screen.getByPlaceholderText('トピックID (例: test-topic)');
      fireEvent.change(input, { target: { value: 'log-test' } });

      const joinButton = screen.getByText('参加');
      fireEvent.click(joinButton);

      // ログタブに切り替え
      const logsTab = screen.getByText('ログ');
      fireEvent.click(logsTab);

      await waitFor(() => {
        expect(screen.getByText(/Joining topic: log-test/)).toBeInTheDocument();
      });
    });

    it('ログをクリアできる', async () => {
      render(<P2PDebugPanel />);

      // 何か操作してログを生成
      const topicsTab = screen.getByText('トピック');
      fireEvent.click(topicsTab);

      const input = screen.getByPlaceholderText('トピックID (例: test-topic)');
      fireEvent.change(input, { target: { value: 'test' } });
      fireEvent.click(screen.getByText('参加'));

      // ログタブに切り替え
      const logsTab = screen.getByText('ログ');
      fireEvent.click(logsTab);

      await waitFor(() => {
        expect(screen.queryByText('ログはありません')).not.toBeInTheDocument();
      });

      const clearButton = screen.getByText('クリア');
      fireEvent.click(clearButton);

      expect(screen.getByText('ログはありません')).toBeInTheDocument();
    });
  });
});
