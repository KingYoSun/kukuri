import { describe, it, expect, vi, beforeEach } from 'vitest';
import { p2pApi } from '@/lib/api/p2p';
import type { P2PMetrics, P2PStatus, TopicStatus } from '@/lib/api/p2p';
import type { CommandResponse } from '@/lib/api/tauriClient';

// Tauri API縺ｮ繝｢繝・け
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

import { invoke } from '@tauri-apps/api/core';

const successResponse = <T>(data: T): CommandResponse<T> => ({
  success: true,
  data,
  error: null,
  error_code: null,
});

describe('p2pApi', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('initialize', () => {
    it('should initialize P2P', async () => {
      vi.mocked(invoke).mockResolvedValueOnce(successResponse(null));

      await p2pApi.initialize();

      expect(invoke).toHaveBeenCalledWith('initialize_p2p');
      expect(invoke).toHaveBeenCalledTimes(1);
    });

    it('should handle initialization error', async () => {
      vi.mocked(invoke).mockRejectedValueOnce(new Error('Failed to initialize'));

      await expect(p2pApi.initialize()).rejects.toThrow('Failed to initialize');
    });
  });

  describe('joinTopic', () => {
    it('should join topic with initial peers', async () => {
      vi.mocked(invoke).mockResolvedValueOnce(successResponse(null));

      await p2pApi.joinTopic('test-topic', ['peer1', 'peer2']);

      expect(invoke).toHaveBeenCalledWith('join_p2p_topic', {
        topicId: 'test-topic',
        initialPeers: ['peer1', 'peer2'],
      });
    });

    it('should join topic without initial peers', async () => {
      vi.mocked(invoke).mockResolvedValueOnce(successResponse(null));

      await p2pApi.joinTopic('test-topic');

      expect(invoke).toHaveBeenCalledWith('join_p2p_topic', {
        topicId: 'test-topic',
        initialPeers: [],
      });
    });
  });

  describe('joinTopicByName', () => {
    it('should join topic by name', async () => {
      vi.mocked(invoke).mockResolvedValueOnce(successResponse(null));

      await p2pApi.joinTopicByName('Bitcoin', ['peer1']);

      expect(invoke).toHaveBeenCalledWith('join_topic_by_name', {
        topicName: 'Bitcoin',
        initialPeers: ['peer1'],
      });
    });
  });

  describe('leaveTopic', () => {
    it('should leave topic', async () => {
      vi.mocked(invoke).mockResolvedValueOnce(successResponse(null));

      await p2pApi.leaveTopic('test-topic');

      expect(invoke).toHaveBeenCalledWith('leave_p2p_topic', {
        topicId: 'test-topic',
      });
    });
  });

  describe('broadcast', () => {
    it('should broadcast message to topic', async () => {
      vi.mocked(invoke).mockResolvedValueOnce(successResponse(null));

      await p2pApi.broadcast('test-topic', 'Hello, P2P!');

      expect(invoke).toHaveBeenCalledWith('broadcast_to_topic', {
        topicId: 'test-topic',
        content: 'Hello, P2P!',
      });
    });
  });

  describe('getStatus', () => {
    it('should get P2P status', async () => {
      const mockStatus: P2PStatus = {
        connected: true,
        endpoint_id: 'node123',
        active_topics: [
          {
            topic_id: 'test-topic',
            peer_count: 5,
            message_count: 100,
            last_activity: Date.now(),
          },
        ],
        peer_count: 10,
        metrics_summary: {
          joins: 3,
          leaves: 1,
          broadcasts_sent: 4,
          messages_received: 6,
        },
      };

      vi.mocked(invoke).mockResolvedValueOnce(successResponse(mockStatus));

      const status = await p2pApi.getStatus();

      expect(invoke).toHaveBeenCalledWith('get_p2p_status');
      expect(status).toEqual(mockStatus);
    });

    it('should handle disconnected status', async () => {
      const mockStatus: P2PStatus = {
        connected: false,
        endpoint_id: '',
        active_topics: [],
        peer_count: 0,
        metrics_summary: {
          joins: 0,
          leaves: 0,
          broadcasts_sent: 0,
          messages_received: 0,
        },
      };

      vi.mocked(invoke).mockResolvedValueOnce(successResponse(mockStatus));

      const status = await p2pApi.getStatus();

      expect(status.connected).toBe(false);
      expect(status.active_topics).toHaveLength(0);
    });
  });

  describe('getNodeAddress', () => {
    it('should get node addresses', async () => {
      const mockAddresses = ['/ip4/192.168.1.1/udp/4001', '/ip6/::1/udp/4001'];

      vi.mocked(invoke).mockResolvedValueOnce(successResponse({ addresses: mockAddresses }));

      const addresses = await p2pApi.getNodeAddress();

      expect(invoke).toHaveBeenCalledWith('get_node_address');
      expect(addresses).toEqual(mockAddresses);
    });

    it('should handle empty addresses', async () => {
      vi.mocked(invoke).mockResolvedValueOnce(successResponse({ addresses: [] }));

      const addresses = await p2pApi.getNodeAddress();

      expect(addresses).toHaveLength(0);
    });
  });

  describe('getMetrics', () => {
    it('should get P2P metrics', async () => {
      const mockMetrics: P2PMetrics = {
        gossip: {
          joins: 3,
          leaves: 1,
          broadcasts_sent: 7,
          messages_received: 12,
          join_details: { total: 3, failures: 1, last_success_ms: 1000, last_failure_ms: 2000 },
          leave_details: { total: 1, failures: 0, last_success_ms: 3000, last_failure_ms: null },
          broadcast_details: {
            total: 7,
            failures: 2,
            last_success_ms: 4000,
            last_failure_ms: 4500,
          },
          receive_details: {
            total: 12,
            failures: 4,
            last_success_ms: 5000,
            last_failure_ms: 5500,
          },
        },
        mainline: {
          connected_peers: 4,
          connection_attempts: 5,
          connection_successes: 4,
          connection_failures: 1,
          connection_last_success_ms: 6000,
          connection_last_failure_ms: 6100,
          routing_attempts: 9,
          routing_successes: 8,
          routing_failures: 1,
          routing_success_rate: 0.888,
          routing_last_success_ms: 6200,
          routing_last_failure_ms: 6300,
          reconnect_attempts: 3,
          reconnect_successes: 2,
          reconnect_failures: 1,
          last_reconnect_success_ms: 6400,
          last_reconnect_failure_ms: 6500,
          bootstrap: {
            env_uses: 1,
            user_uses: 2,
            bundle_uses: 0,
            fallback_uses: 0,
            last_source: 'user',
            last_applied_ms: 7000,
          },
        },
      };
      vi.mocked(invoke).mockResolvedValueOnce(successResponse(mockMetrics));

      const metrics = await p2pApi.getMetrics();

      expect(invoke).toHaveBeenCalledWith('get_p2p_metrics');
      expect(metrics).toEqual(mockMetrics);
    });
  });

  describe('connectToPeer', () => {
    it('should connect to a peer with valid address', async () => {
      vi.mocked(invoke).mockResolvedValueOnce(successResponse(null));

      await p2pApi.connectToPeer('/ip4/192.168.1.100/tcp/4001/p2p/12D3KooWExample');

      expect(invoke).toHaveBeenCalledWith('connect_to_peer', {
        peerAddress: '/ip4/192.168.1.100/tcp/4001/p2p/12D3KooWExample',
      });
    });

    it('should handle connection error', async () => {
      vi.mocked(invoke).mockRejectedValueOnce(new Error('Connection refused'));

      await expect(
        p2pApi.connectToPeer('/ip4/192.168.1.100/tcp/4001/p2p/12D3KooWExample'),
      ).rejects.toThrow('Connection refused');
    });

    it('should connect to IPv6 peer', async () => {
      vi.mocked(invoke).mockResolvedValueOnce(successResponse(null));

      await p2pApi.connectToPeer('/ip6/2001:db8::1/tcp/4001/p2p/12D3KooWExample');

      expect(invoke).toHaveBeenCalledWith('connect_to_peer', {
        peerAddress: '/ip6/2001:db8::1/tcp/4001/p2p/12D3KooWExample',
      });
    });
  });

  describe('Error handling', () => {
    it('should propagate errors from Tauri commands', async () => {
      const errorMessage = 'P2P manager not initialized';
      vi.mocked(invoke).mockRejectedValueOnce(new Error(errorMessage));

      await expect(p2pApi.joinTopic('test-topic')).rejects.toThrow(errorMessage);
    });
  });

  describe('Type validation', () => {
    it('should validate TopicStatus type', () => {
      const topicStatus: TopicStatus = {
        topic_id: 'test-topic',
        peer_count: 5,
        message_count: 100,
        last_activity: 1234567890,
      };

      expect(topicStatus.topic_id).toBe('test-topic');
      expect(topicStatus.peer_count).toBe(5);
      expect(topicStatus.message_count).toBe(100);
      expect(topicStatus.last_activity).toBe(1234567890);
    });

    it('should validate P2PStatus type', () => {
      const p2pStatus: P2PStatus = {
        connected: true,
        endpoint_id: 'node123',
        active_topics: [],
        peer_count: 0,
        metrics_summary: {
          joins: 0,
          leaves: 0,
          broadcasts_sent: 0,
          messages_received: 0,
        },
      };

      expect(p2pStatus.connected).toBe(true);
      expect(p2pStatus.endpoint_id).toBe('node123');
      expect(p2pStatus.active_topics).toHaveLength(0);
      expect(p2pStatus.peer_count).toBe(0);
    });
  });
});
