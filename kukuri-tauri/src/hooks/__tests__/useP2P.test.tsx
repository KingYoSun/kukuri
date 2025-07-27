import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest'
import { act, renderHook, waitFor } from '@testing-library/react'
import { useP2P } from '../useP2P'
import { useP2PStore } from '@/stores/p2pStore'
import * as p2pApi from '@/lib/api/p2p'

// P2P APIのモック
vi.mock('@/lib/api/p2p', () => ({
  p2pApi: {
    initialize: vi.fn(),
    getNodeAddress: vi.fn(),
    getStatus: vi.fn(),
    joinTopic: vi.fn(),
    leaveTopic: vi.fn(),
    broadcast: vi.fn(),
  }
}))

// P2PEventListenerのモック
vi.mock('../useP2PEventListener', () => ({
  useP2PEventListener: vi.fn(),
}))

describe('useP2P', () => {
  beforeEach(() => {
    // ストアをリセット
    const { result } = renderHook(() => useP2PStore())
    act(() => {
      result.current.reset()
    })
    
    // モックをリセット
    vi.clearAllMocks()
  })

  afterEach(() => {
    vi.clearAllTimers()
  })

  describe('自動初期化', () => {
    it('未初期化の場合、自動的に初期化を開始する', async () => {
      vi.mocked(p2pApi.p2pApi.initialize).mockResolvedValueOnce(undefined)
      vi.mocked(p2pApi.p2pApi.getNodeAddress).mockResolvedValueOnce(['/ip4/127.0.0.1/tcp/4001'])
      vi.mocked(p2pApi.p2pApi.getStatus).mockResolvedValueOnce({
        connected: true,
        endpoint_id: 'node123',
        active_topics: [],
        peer_count: 0,
      })

      const { result } = renderHook(() => useP2P())

      expect(result.current.initialized).toBe(false)

      await waitFor(() => {
        expect(result.current.initialized).toBe(true)
        expect(result.current.connectionStatus).toBe('connected')
      })
    })
  })

  describe('定期的な状態更新', () => {
    it('接続中は定期的に状態を更新する', async () => {
      vi.useFakeTimers()

      // 初期化を成功させる
      vi.mocked(p2pApi.p2pApi.initialize).mockResolvedValueOnce(undefined)
      vi.mocked(p2pApi.p2pApi.getNodeAddress).mockResolvedValueOnce(['/ip4/127.0.0.1/tcp/4001'])
      vi.mocked(p2pApi.p2pApi.getStatus).mockResolvedValue({
        connected: true,
        endpoint_id: 'node123',
        active_topics: [
          {
            topic_id: 'topic1',
            peer_count: 2,
            message_count: 10,
            last_activity: Date.now(),
          }
        ],
        peer_count: 2,
      })

      const { result } = renderHook(() => useP2P())

      // 初期化を待つ
      await waitFor(() => {
        expect(result.current.initialized).toBe(true)
      })

      // 状態更新が呼ばれることを確認
      expect(vi.mocked(p2pApi.p2pApi.getStatus)).toHaveBeenCalledTimes(1)

      // 30秒進める
      act(() => {
        vi.advanceTimersByTime(30000)
      })

      await waitFor(() => {
        expect(vi.mocked(p2pApi.p2pApi.getStatus)).toHaveBeenCalledTimes(2)
      })

      vi.useRealTimers()
    })
  })

  describe('ヘルパー関数', () => {
    it('getTopicMessages - トピックのメッセージを取得できる', () => {
      const { result } = renderHook(() => useP2P())

      const message = {
        id: 'msg1',
        topic_id: 'topic1',
        author: 'author1',
        content: 'Test message',
        timestamp: Date.now(),
        signature: 'sig1',
      }

      // ストアに直接メッセージを追加
      act(() => {
        useP2PStore.getState().addMessage(message)
      })

      const messages = result.current.getTopicMessages('topic1')
      expect(messages).toHaveLength(1)
      expect(messages[0]).toEqual(message)
    })

    it('getTopicStats - トピックの統計情報を取得できる', () => {
      const { result } = renderHook(() => useP2P())

      const stats = {
        topic_id: 'topic1',
        peer_count: 5,
        message_count: 100,
        recent_messages: [],
        connected_peers: ['peer1', 'peer2'],
      }

      // ストアに直接統計情報を追加
      act(() => {
        useP2PStore.getState().activeTopics.set('topic1', stats)
      })

      const topicStats = result.current.getTopicStats('topic1')
      expect(topicStats).toEqual(stats)
    })

    it('isJoinedTopic - トピック参加状態を確認できる', () => {
      const { result } = renderHook(() => useP2P())

      act(() => {
        useP2PStore.getState().activeTopics.set('topic1', {
          topic_id: 'topic1',
          peer_count: 1,
          message_count: 0,
          recent_messages: [],
          connected_peers: [],
        })
      })

      expect(result.current.isJoinedTopic('topic1')).toBe(true)
      expect(result.current.isJoinedTopic('topic2')).toBe(false)
    })

    it('getConnectedPeerCount - 接続中のピア数を取得できる', () => {
      const { result } = renderHook(() => useP2P())

      act(() => {
        const peers = useP2PStore.getState().peers
        peers.set('peer1', {
          node_id: 'peer1',
          node_addr: 'addr1',
          topics: [],
          last_seen: Date.now(),
          connection_status: 'connected',
        })
        peers.set('peer2', {
          node_id: 'peer2',
          node_addr: 'addr2',
          topics: [],
          last_seen: Date.now(),
          connection_status: 'disconnected',
        })
        peers.set('peer3', {
          node_id: 'peer3',
          node_addr: 'addr3',
          topics: [],
          last_seen: Date.now(),
          connection_status: 'connected',
        })
      })

      expect(result.current.getConnectedPeerCount()).toBe(2)
    })

    it('getTopicPeerCount - トピックのピア数を取得できる', () => {
      const { result } = renderHook(() => useP2P())

      act(() => {
        useP2PStore.getState().activeTopics.set('topic1', {
          topic_id: 'topic1',
          peer_count: 10,
          message_count: 0,
          recent_messages: [],
          connected_peers: [],
        })
      })

      expect(result.current.getTopicPeerCount('topic1')).toBe(10)
      expect(result.current.getTopicPeerCount('topic2')).toBe(0)
    })
  })

  describe('アクション', () => {
    it('joinTopic - トピックに参加できる', async () => {
      vi.mocked(p2pApi.p2pApi.joinTopic).mockResolvedValueOnce(undefined)
      vi.mocked(p2pApi.p2pApi.getStatus).mockResolvedValueOnce({
        connected: true,
        endpoint_id: 'node123',
        active_topics: [
          {
            topic_id: 'new-topic',
            peer_count: 1,
            message_count: 0,
            last_activity: Date.now(),
          }
        ],
        peer_count: 1,
      })

      const { result } = renderHook(() => useP2P())

      await act(async () => {
        await result.current.joinTopic('new-topic', ['initial-peer'])
      })

      expect(vi.mocked(p2pApi.p2pApi.joinTopic)).toHaveBeenCalledWith('new-topic', ['initial-peer'])
    })

    it('leaveTopic - トピックから離脱できる', async () => {
      vi.mocked(p2pApi.p2pApi.leaveTopic).mockResolvedValueOnce(undefined)

      const { result } = renderHook(() => useP2P())

      await act(async () => {
        await result.current.leaveTopic('topic1')
      })

      expect(vi.mocked(p2pApi.p2pApi.leaveTopic)).toHaveBeenCalledWith('topic1')
    })

    it('broadcast - メッセージをブロードキャストできる', async () => {
      vi.mocked(p2pApi.p2pApi.broadcast).mockResolvedValueOnce(undefined)

      const { result } = renderHook(() => useP2P())

      await act(async () => {
        await result.current.broadcast('topic1', 'Hello world!')
      })

      expect(vi.mocked(p2pApi.p2pApi.broadcast)).toHaveBeenCalledWith('topic1', 'Hello world!')
    })

    it('clearError - エラーをクリアできる', () => {
      const { result } = renderHook(() => useP2P())

      // エラーを設定
      act(() => {
        useP2PStore.getState().error = 'Test error'
      })

      expect(result.current.error).toBe('Test error')

      act(() => {
        result.current.clearError()
      })

      expect(result.current.error).toBe(null)
    })
  })
})