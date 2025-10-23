import { describe, it, expect, beforeEach, vi } from 'vitest';
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

describe('p2pStore', () => {
  beforeEach(() => {
    // モックをリセット
    vi.clearAllMocks();
    // ストアの状態をリセット
    act(() => {
      useP2PStore.setState({
        initialized: false,
        nodeId: null,
        nodeAddr: null,
        activeTopics: new Map(),
        peers: new Map(),
        messages: new Map(),
        connectionStatus: 'disconnected',
        error: null,
        metricsSummary: null,
      });
    });
  });

  describe('initialize', () => {
    it('P2P機能を正常に初期化できる', async () => {
      const mockNodeAddr = ['/ip4/127.0.0.1/tcp/4001/p2p/QmNodeId123'];
      const mockStatus = {
        connected: true,
        endpoint_id: 'QmNodeId123',
        active_topics: [],
        peer_count: 0,
        metrics_summary: {
          joins: 0,
          leaves: 0,
          broadcasts_sent: 0,
          messages_received: 0,
        },
      };

      vi.mocked(p2pApi.initialize).mockResolvedValueOnce(undefined);
      vi.mocked(p2pApi.getNodeAddress).mockResolvedValueOnce(mockNodeAddr);
      vi.mocked(p2pApi.getStatus).mockResolvedValueOnce(mockStatus);

      expect(useP2PStore.getState().initialized).toBe(false);
      expect(useP2PStore.getState().connectionStatus).toBe('disconnected');

      await act(async () => {
        await useP2PStore.getState().initialize();
      });

      expect(useP2PStore.getState().initialized).toBe(true);
      expect(useP2PStore.getState().nodeId).toBe('QmNodeId123');
      expect(useP2PStore.getState().nodeAddr).toBe(mockNodeAddr.join(', '));
      expect(useP2PStore.getState().connectionStatus).toBe('connected');
      expect(useP2PStore.getState().metricsSummary).toEqual({
        joins: 0,
        leaves: 0,
        broadcasts_sent: 0,
        messages_received: 0,
      });
    });

    it('初期化エラーを適切に処理する', async () => {
      const mockError = new Error('Failed to initialize P2P');
      vi.mocked(p2pApi.initialize).mockRejectedValueOnce(mockError);

      await act(async () => {
        await useP2PStore.getState().initialize();
      });

      expect(useP2PStore.getState().initialized).toBe(false);
      expect(useP2PStore.getState().connectionStatus).toBe('error');
      expect(useP2PStore.getState().error).toBe('Failed to initialize P2P');
    });
  });

  describe('joinTopic', () => {
    it('トピックに正常に参加できる', async () => {
      vi.mocked(p2pApi.joinTopic).mockResolvedValueOnce(undefined);
      vi.mocked(p2pApi.getStatus).mockResolvedValueOnce({
        connected: true,
        endpoint_id: 'QmNodeId123',
        active_topics: [
          {
            topic_id: 'test-topic',
            peer_count: 3,
            message_count: 10,
            last_activity: Date.now(),
          },
        ],
        peer_count: 3,
        metrics_summary: {
          joins: 1,
          leaves: 0,
          broadcasts_sent: 2,
          messages_received: 3,
        },
      });

      await act(async () => {
        await useP2PStore.getState().joinTopic('test-topic', ['initial-peer']);
      });

      expect(vi.mocked(p2pApi.joinTopic)).toHaveBeenCalledWith('test-topic', ['initial-peer']);

      const topicStats = useP2PStore.getState().activeTopics.get('test-topic');
      expect(topicStats).toBeDefined();
      expect(topicStats?.topic_id).toBe('test-topic');
      expect(topicStats?.peer_count).toBe(3);
      expect(useP2PStore.getState().metricsSummary).toEqual({
        joins: 1,
        leaves: 0,
        broadcasts_sent: 2,
        messages_received: 3,
      });
    });

    it('トピック参加エラーを適切に処理する', async () => {
      const mockError = new Error('Failed to join topic');
      vi.mocked(p2pApi.joinTopic).mockRejectedValueOnce(mockError);

      await act(async () => {
        await useP2PStore.getState().joinTopic('test-topic');
      });

      expect(useP2PStore.getState().error).toBe('Failed to join topic');
    });
  });

  describe('leaveTopic', () => {
    it('トピックから正常に離脱できる', async () => {
      vi.mocked(p2pApi.joinTopic).mockResolvedValueOnce(undefined);
      vi.mocked(p2pApi.leaveTopic).mockResolvedValueOnce(undefined);
      vi.mocked(p2pApi.getStatus).mockResolvedValueOnce({
        connected: true,
        endpoint_id: 'QmNodeId123',
        active_topics: [
          {
            topic_id: 'test-topic',
            peer_count: 0,
            message_count: 0,
            last_activity: Date.now(),
          },
        ],
        peer_count: 0,
        metrics_summary: {
          joins: 0,
          leaves: 1,
          broadcasts_sent: 0,
          messages_received: 0,
        },
      });

      // 事前にトピックに参加
      await act(async () => {
        await useP2PStore.getState().joinTopic('test-topic');
      });

      await act(async () => {
        await useP2PStore.getState().leaveTopic('test-topic');
      });

      expect(vi.mocked(p2pApi.leaveTopic)).toHaveBeenCalledWith('test-topic');
      expect(useP2PStore.getState().activeTopics.has('test-topic')).toBe(false);
      expect(useP2PStore.getState().metricsSummary).toEqual({
        joins: 0,
        leaves: 1,
        broadcasts_sent: 0,
        messages_received: 0,
      });
      expect(useP2PStore.getState().messages.has('test-topic')).toBe(false);
    });
  });

  describe('broadcast', () => {
    it('メッセージを正常にブロードキャストできる', async () => {
      vi.mocked(p2pApi.broadcast).mockResolvedValueOnce(undefined);

      await act(async () => {
        await useP2PStore.getState().broadcast('test-topic', 'Hello P2P!');
      });

      expect(vi.mocked(p2pApi.broadcast)).toHaveBeenCalledWith('test-topic', 'Hello P2P!');
    });

    it('ブロードキャストエラーを適切に処理する', async () => {
      const mockError = new Error('Failed to broadcast');
      vi.mocked(p2pApi.broadcast).mockRejectedValueOnce(mockError);

      await act(async () => {
        await useP2PStore.getState().broadcast('test-topic', 'Hello P2P!');
      });

      expect(useP2PStore.getState().error).toBe('Failed to broadcast');
    });
  });

  describe('addMessage', () => {
    it('新しいメッセージを追加できる', () => {
      const message = {
        id: 'msg1',
        topic_id: 'test-topic',
        author: 'author1',
        content: 'Test message',
        timestamp: Date.now(),
        signature: 'sig1',
      };

      act(() => {
        useP2PStore.getState().addMessage(message);
      });

      const topicMessages = useP2PStore.getState().messages.get('test-topic');
      expect(topicMessages).toHaveLength(1);
      expect(topicMessages?.[0]).toEqual(message);
    });

    it('重複メッセージを追加しない', () => {
      const message = {
        id: 'msg1',
        topic_id: 'test-topic',
        author: 'author1',
        content: 'Test message',
        timestamp: Date.now(),
        signature: 'sig1',
      };

      act(() => {
        useP2PStore.getState().addMessage(message);
        useP2PStore.getState().addMessage(message); // 同じメッセージを再度追加
      });

      const topicMessages = useP2PStore.getState().messages.get('test-topic');
      expect(topicMessages).toHaveLength(1);
    });
  });

  describe('updatePeer', () => {
    it('ピア情報を更新できる', () => {
      const peer = {
        node_id: 'peer1',
        node_addr: '/ip4/192.168.1.1/tcp/4001',
        topics: ['topic1', 'topic2'],
        last_seen: Date.now(),
        connection_status: 'connected' as const,
      };

      act(() => {
        useP2PStore.getState().updatePeer(peer);
      });

      const storedPeer = useP2PStore.getState().peers.get('peer1');
      expect(storedPeer).toEqual(peer);
    });
  });

  describe('removePeer', () => {
    it('ピアを削除できる', () => {
      // 事前にピアを追加
      const peer = {
        node_id: 'peer1',
        node_addr: '/ip4/192.168.1.1/tcp/4001',
        topics: ['topic1'],
        last_seen: Date.now(),
        connection_status: 'connected' as const,
      };

      act(() => {
        useP2PStore.getState().updatePeer(peer);
        useP2PStore.getState().removePeer('peer1');
      });

      expect(useP2PStore.getState().peers.has('peer1')).toBe(false);
    });
  });

  describe('clearError', () => {
    it('エラーをクリアできる', () => {
      // エラーを設定
      act(() => {
        useP2PStore.setState({ error: 'Test error' });
      });

      expect(useP2PStore.getState().error).toBe('Test error');

      act(() => {
        useP2PStore.getState().clearError();
      });

      expect(useP2PStore.getState().error).toBe(null);
    });
  });

  describe('reset', () => {
    it('ストアを初期状態にリセットできる', () => {
      // データを設定
      act(() => {
        const activeTopics = new Map();
        activeTopics.set('topic1', {
          topic_id: 'topic1',
          peer_count: 1,
          message_count: 1,
          recent_messages: [],
          connected_peers: [],
        });

        useP2PStore.setState({
          initialized: true,
          nodeId: 'node123',
          nodeAddr: '/ip4/127.0.0.1/tcp/4001',
          connectionStatus: 'connected',
          activeTopics,
        });
      });

      act(() => {
        useP2PStore.getState().reset();
      });

      expect(useP2PStore.getState().initialized).toBe(false);
      expect(useP2PStore.getState().nodeId).toBe(null);
      expect(useP2PStore.getState().nodeAddr).toBe(null);
      expect(useP2PStore.getState().connectionStatus).toBe('disconnected');
      expect(useP2PStore.getState().activeTopics.size).toBe(0);
      expect(useP2PStore.getState().metricsSummary).toBeNull();
    });
  });
});
