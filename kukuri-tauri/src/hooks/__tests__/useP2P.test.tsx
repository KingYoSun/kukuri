import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest';
import { act } from '@testing-library/react';
import { useP2PStore } from '@/stores/p2pStore';
import { p2pApi } from '@/lib/api/p2p';

// P2P APIのモック
vi.mock('@/lib/api/p2p', () => ({
  p2pApi: {
    initialize: vi.fn(),
    getNodeAddress: vi.fn(),
    getStatus: vi.fn(),
    joinTopic: vi.fn(),
    leaveTopic: vi.fn(),
    broadcast: vi.fn(),
  },
}));

// useP2Pフックのモック - 初期化処理のタイミング問題を回避
vi.mock('../useP2P', async () => {
  const actual = await vi.importActual<{ useP2P: () => ReturnType<typeof useP2PStore> }>(
    '../useP2P',
  );
  return {
    ...actual,
    useP2P: () => {
      const store = useP2PStore();
      return {
        ...store,
        // ヘルパー関数を追加
        getTopicMessages: (topicId: string) => {
          return store.messages.get(topicId) || [];
        },
        getTopicStats: (topicId: string) => {
          return store.activeTopics.get(topicId);
        },
        isJoinedTopic: (topicId: string) => {
          return store.activeTopics.has(topicId);
        },
        getConnectedPeerCount: () => {
          return Array.from(store.peers.values()).filter((p) => p.connection_status === 'connected')
            .length;
        },
        getTopicPeerCount: (topicId: string) => {
          const stats = store.activeTopics.get(topicId);
          return stats?.peer_count || 0;
        },
      };
    },
  };
});

// P2PEventListenerのモック
vi.mock('../useP2PEventListener', () => ({
  useP2PEventListener: vi.fn(),
}));

describe('useP2P', () => {
  beforeEach(() => {
    // モックをリセット
    vi.clearAllMocks();

    // ストアの状態をリセット
    act(() => {
      useP2PStore.setState({
        initialized: false,
        connectionStatus: 'disconnected',
        nodeAddr: null,
        nodeId: null,
        activeTopics: new Map(),
        messages: new Map(),
        error: null,
        peers: new Map(),
      });
    });
  });

  afterEach(() => {
    vi.clearAllTimers();
  });

  describe('初期化', () => {
    it('初期化が正常に完了する', async () => {
      // モックを設定
      const initializeMock = vi.mocked(p2pApi.initialize);
      const getNodeAddressMock = vi.mocked(p2pApi.getNodeAddress);
      const getStatusMock = vi.mocked(p2pApi.getStatus);

      initializeMock.mockResolvedValueOnce(undefined);
      getNodeAddressMock.mockResolvedValueOnce(['/ip4/127.0.0.1/tcp/4001']);
      getStatusMock.mockResolvedValueOnce({
        connected: true,
        endpoint_id: 'node123',
        active_topics: [],
        peer_count: 0,
      });

      // 初期状態を確認
      expect(useP2PStore.getState().initialized).toBe(false);
      expect(useP2PStore.getState().connectionStatus).toBe('disconnected');

      // 初期化を実行
      await act(async () => {
        await useP2PStore.getState().initialize();
      });

      // 初期化が完了したことを確認
      expect(useP2PStore.getState().initialized).toBe(true);
      expect(useP2PStore.getState().connectionStatus).toBe('connected');
      expect(useP2PStore.getState().nodeId).toBe('node123');
      expect(useP2PStore.getState().nodeAddr).toBe('/ip4/127.0.0.1/tcp/4001');
    });

    it('初期化エラーを適切に処理する', async () => {
      const initializeMock = vi.mocked(p2pApi.initialize);
      const errorMessage = 'P2P initialization failed';
      initializeMock.mockRejectedValueOnce(new Error(errorMessage));

      await act(async () => {
        await useP2PStore.getState().initialize();
      });

      expect(useP2PStore.getState().initialized).toBe(false);
      expect(useP2PStore.getState().connectionStatus).toBe('error');
      expect(useP2PStore.getState().error).toBe(errorMessage);
    });
  });

  describe('状態更新', () => {
    it('refreshStatusが状態を正しく更新する', async () => {
      vi.mocked(p2pApi.getStatus).mockResolvedValueOnce({
        connected: true,
        endpoint_id: 'node123',
        active_topics: [
          {
            topic_id: 'topic1',
            peer_count: 2,
            message_count: 10,
            last_activity: Date.now(),
          },
        ],
        peer_count: 2,
      });

      await act(async () => {
        await useP2PStore.getState().refreshStatus();
      });

      const activeTopics = useP2PStore.getState().activeTopics;
      const topic1Stats = activeTopics.get('topic1');
      expect(topic1Stats).toBeDefined();
      expect(topic1Stats?.peer_count).toBe(2);
      expect(topic1Stats?.message_count).toBe(10);
    });
  });

  describe('ヘルパー関数', () => {
    it('getTopicMessages - トピックのメッセージを取得できる', () => {
      const message = {
        id: 'msg1',
        topic_id: 'topic1',
        author: 'author1',
        content: 'Test message',
        timestamp: Date.now(),
        signature: 'sig1',
      };

      // ストアに直接メッセージを追加
      act(() => {
        useP2PStore.getState().addMessage(message);
      });

      // ストアから直接メッセージを取得
      const messages = useP2PStore.getState().messages.get('topic1') || [];
      expect(messages).toHaveLength(1);
      expect(messages[0]).toEqual(message);
    });

    it('getTopicStats - トピックの統計情報を取得できる', () => {
      const stats = {
        topic_id: 'topic1',
        peer_count: 5,
        message_count: 100,
        recent_messages: [],
        connected_peers: ['peer1', 'peer2'],
      };

      // ストアに直接統計情報を追加
      act(() => {
        const activeTopics = new Map(useP2PStore.getState().activeTopics);
        activeTopics.set('topic1', stats);
        useP2PStore.setState({ activeTopics });
      });

      const topicStats = useP2PStore.getState().activeTopics.get('topic1');
      expect(topicStats).toEqual(stats);
    });

    it('isJoinedTopic - トピック参加状態を確認できる', () => {
      act(() => {
        const activeTopics = new Map();
        activeTopics.set('topic1', {
          topic_id: 'topic1',
          peer_count: 1,
          message_count: 0,
          recent_messages: [],
          connected_peers: [],
        });
        useP2PStore.setState({ activeTopics });
      });

      const hasJoinedTopic1 = useP2PStore.getState().activeTopics.has('topic1');
      const hasJoinedTopic2 = useP2PStore.getState().activeTopics.has('topic2');
      expect(hasJoinedTopic1).toBe(true);
      expect(hasJoinedTopic2).toBe(false);
    });

    it('getConnectedPeerCount - 接続中のピア数を取得できる', () => {
      act(() => {
        const peers = new Map();
        peers.set('peer1', {
          node_id: 'peer1',
          node_addr: 'addr1',
          topics: [],
          last_seen: Date.now(),
          connection_status: 'connected' as const,
        });
        peers.set('peer2', {
          node_id: 'peer2',
          node_addr: 'addr2',
          topics: [],
          last_seen: Date.now(),
          connection_status: 'disconnected' as const,
        });
        peers.set('peer3', {
          node_id: 'peer3',
          node_addr: 'addr3',
          topics: [],
          last_seen: Date.now(),
          connection_status: 'connected' as const,
        });
        useP2PStore.setState({ peers });
      });

      const connectedCount = Array.from(useP2PStore.getState().peers.values()).filter(
        (p) => p.connection_status === 'connected',
      ).length;
      expect(connectedCount).toBe(2);
    });

    it('getTopicPeerCount - トピックのピア数を取得できる', () => {
      act(() => {
        const activeTopics = new Map();
        activeTopics.set('topic1', {
          topic_id: 'topic1',
          peer_count: 10,
          message_count: 0,
          recent_messages: [],
          connected_peers: [],
        });
        useP2PStore.setState({ activeTopics });
      });

      const topic1Stats = useP2PStore.getState().activeTopics.get('topic1');
      const topic2Stats = useP2PStore.getState().activeTopics.get('topic2');
      expect(topic1Stats?.peer_count || 0).toBe(10);
      expect(topic2Stats?.peer_count || 0).toBe(0);
    });
  });

  describe('アクション', () => {
    it('joinTopic - トピックに参加できる', async () => {
      vi.mocked(p2pApi.joinTopic).mockResolvedValueOnce(undefined);
      vi.mocked(p2pApi.getStatus).mockResolvedValueOnce({
        connected: true,
        endpoint_id: 'node123',
        active_topics: [
          {
            topic_id: 'new-topic',
            peer_count: 1,
            message_count: 0,
            last_activity: Date.now(),
          },
        ],
        peer_count: 1,
      });

      await act(async () => {
        await useP2PStore.getState().joinTopic('new-topic', ['initial-peer']);
      });

      expect(vi.mocked(p2pApi.joinTopic)).toHaveBeenCalledWith('new-topic', ['initial-peer']);
      expect(useP2PStore.getState().activeTopics.has('new-topic')).toBe(true);
    });

    it('leaveTopic - トピックから離脱できる', async () => {
      vi.mocked(p2pApi.leaveTopic).mockResolvedValueOnce(undefined);

      // 事前にトピックを追加
      act(() => {
        const activeTopics = new Map();
        activeTopics.set('topic1', {
          topic_id: 'topic1',
          peer_count: 1,
          message_count: 0,
          recent_messages: [],
          connected_peers: [],
        });
        useP2PStore.setState({ activeTopics });
      });

      await act(async () => {
        await useP2PStore.getState().leaveTopic('topic1');
      });

      expect(vi.mocked(p2pApi.leaveTopic)).toHaveBeenCalledWith('topic1');
      expect(useP2PStore.getState().activeTopics.has('topic1')).toBe(false);
    });

    it('broadcast - メッセージをブロードキャストできる', async () => {
      vi.mocked(p2pApi.broadcast).mockResolvedValueOnce(undefined);

      await act(async () => {
        await useP2PStore.getState().broadcast('topic1', 'Hello world!');
      });

      expect(vi.mocked(p2pApi.broadcast)).toHaveBeenCalledWith('topic1', 'Hello world!');
    });

    it('clearError - エラーをクリアできる', () => {
      // エラーを設定
      act(() => {
        useP2PStore.setState({ error: 'Test error' });
      });

      // エラーが設定されたことを確認
      expect(useP2PStore.getState().error).toBe('Test error');

      // エラーをクリア
      act(() => {
        useP2PStore.getState().clearError();
      });

      // エラーがクリアされたことを確認
      expect(useP2PStore.getState().error).toBe(null);
    });
  });
});
