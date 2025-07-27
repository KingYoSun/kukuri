import { describe, it, expect, beforeEach, vi } from 'vitest'
import { act, renderHook } from '@testing-library/react'
import { useP2PStore } from '../p2pStore'
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

describe('p2pStore', () => {
  beforeEach(() => {
    // モックをリセット
    vi.clearAllMocks()
    // ストアの状態をリセット
    useP2PStore.setState({
      initialized: false,
      nodeId: null,
      nodeAddr: null,
      activeTopics: new Map(),
      peers: new Map(),
      messages: new Map(),
      connectionStatus: 'disconnected',
      error: null,
    })
  })

  describe('initialize', () => {
    it('P2P機能を正常に初期化できる', async () => {
      const mockNodeAddr = ['/ip4/127.0.0.1/tcp/4001/p2p/QmNodeId123']
      const mockStatus = {
        connected: true,
        endpoint_id: 'QmNodeId123',
        active_topics: [],
        peer_count: 0,
      }

      vi.mocked(p2pApi.p2pApi.initialize).mockResolvedValueOnce(undefined)
      vi.mocked(p2pApi.p2pApi.getNodeAddress).mockResolvedValueOnce(mockNodeAddr)
      vi.mocked(p2pApi.p2pApi.getStatus).mockResolvedValueOnce(mockStatus)

      const { result } = renderHook(() => useP2PStore())

      expect(result.current.initialized).toBe(false)
      expect(result.current.connectionStatus).toBe('disconnected')

      await act(async () => {
        await result.current.initialize()
      })

      expect(result.current.initialized).toBe(true)
      expect(result.current.nodeId).toBe('QmNodeId123')
      expect(result.current.nodeAddr).toBe(mockNodeAddr.join(', '))
      expect(result.current.connectionStatus).toBe('connected')
    })

    it('初期化エラーを適切に処理する', async () => {
      const mockError = new Error('Failed to initialize P2P')
      vi.mocked(p2pApi.p2pApi.initialize).mockRejectedValueOnce(mockError)

      const { result } = renderHook(() => useP2PStore())

      await act(async () => {
        await result.current.initialize()
      })

      expect(result.current.initialized).toBe(false)
      expect(result.current.connectionStatus).toBe('error')
      expect(result.current.error).toBe('Failed to initialize P2P')
    })
  })

  describe('joinTopic', () => {
    it('トピックに正常に参加できる', async () => {
      vi.mocked(p2pApi.p2pApi.joinTopic).mockResolvedValueOnce(undefined)
      vi.mocked(p2pApi.p2pApi.getStatus).mockResolvedValueOnce({
        connected: true,
        endpoint_id: 'QmNodeId123',
        active_topics: [
          {
            topic_id: 'test-topic',
            peer_count: 3,
            message_count: 10,
            last_activity: Date.now(),
          }
        ],
        peer_count: 3
      })

      const { result } = renderHook(() => useP2PStore())

      await act(async () => {
        await result.current.joinTopic('test-topic', ['initial-peer'])
      })

      expect(vi.mocked(p2pApi.p2pApi.joinTopic)).toHaveBeenCalledWith('test-topic', ['initial-peer'])
      
      const topicStats = result.current.activeTopics.get('test-topic')
      expect(topicStats).toBeDefined()
      expect(topicStats?.topic_id).toBe('test-topic')
      expect(topicStats?.peer_count).toBe(3)
    })

    it('トピック参加エラーを適切に処理する', async () => {
      const mockError = new Error('Failed to join topic')
      vi.mocked(p2pApi.p2pApi.joinTopic).mockRejectedValueOnce(mockError)

      const { result } = renderHook(() => useP2PStore())

      await act(async () => {
        await result.current.joinTopic('test-topic')
      })

      expect(result.current.error).toBe('Failed to join topic')
    })
  })

  describe('leaveTopic', () => {
    it('トピックから正常に離脱できる', async () => {
      vi.mocked(p2pApi.p2pApi.joinTopic).mockResolvedValueOnce(undefined)
      vi.mocked(p2pApi.p2pApi.leaveTopic).mockResolvedValueOnce(undefined)

      const { result } = renderHook(() => useP2PStore())

      // 事前にトピックに参加
      await act(async () => {
        await result.current.joinTopic('test-topic')
      })

      await act(async () => {
        await result.current.leaveTopic('test-topic')
      })

      expect(vi.mocked(p2pApi.p2pApi.leaveTopic)).toHaveBeenCalledWith('test-topic')
      expect(result.current.activeTopics.has('test-topic')).toBe(false)
      expect(result.current.messages.has('test-topic')).toBe(false)
    })
  })

  describe('broadcast', () => {
    it('メッセージを正常にブロードキャストできる', async () => {
      vi.mocked(p2pApi.p2pApi.broadcast).mockResolvedValueOnce(undefined)

      const { result } = renderHook(() => useP2PStore())

      await act(async () => {
        await result.current.broadcast('test-topic', 'Hello P2P!')
      })

      expect(vi.mocked(p2pApi.p2pApi.broadcast)).toHaveBeenCalledWith('test-topic', 'Hello P2P!')
    })

    it('ブロードキャストエラーを適切に処理する', async () => {
      const mockError = new Error('Failed to broadcast')
      vi.mocked(p2pApi.p2pApi.broadcast).mockRejectedValueOnce(mockError)

      const { result } = renderHook(() => useP2PStore())

      await act(async () => {
        await result.current.broadcast('test-topic', 'Hello P2P!')
      })

      expect(result.current.error).toBe('Failed to broadcast message')
    })
  })

  describe('addMessage', () => {
    it('新しいメッセージを追加できる', () => {
      const { result } = renderHook(() => useP2PStore())

      const message = {
        id: 'msg1',
        topic_id: 'test-topic',
        author: 'author1',
        content: 'Test message',
        timestamp: Date.now(),
        signature: 'sig1',
      }

      act(() => {
        result.current.addMessage(message)
      })

      const topicMessages = result.current.messages.get('test-topic')
      expect(topicMessages).toHaveLength(1)
      expect(topicMessages?.[0]).toEqual(message)
    })

    it('重複メッセージを追加しない', () => {
      const { result } = renderHook(() => useP2PStore())

      const message = {
        id: 'msg1',
        topic_id: 'test-topic',
        author: 'author1',
        content: 'Test message',
        timestamp: Date.now(),
        signature: 'sig1',
      }

      act(() => {
        result.current.addMessage(message)
        result.current.addMessage(message) // 同じメッセージを再度追加
      })

      const topicMessages = result.current.messages.get('test-topic')
      expect(topicMessages).toHaveLength(1)
    })
  })

  describe('updatePeer', () => {
    it('ピア情報を更新できる', () => {
      const { result } = renderHook(() => useP2PStore())

      const peer = {
        node_id: 'peer1',
        node_addr: '/ip4/192.168.1.1/tcp/4001',
        topics: ['topic1', 'topic2'],
        last_seen: Date.now(),
        connection_status: 'connected' as const,
      }

      act(() => {
        result.current.updatePeer(peer)
      })

      const storedPeer = result.current.peers.get('peer1')
      expect(storedPeer).toEqual(peer)
    })
  })

  describe('removePeer', () => {
    it('ピアを削除できる', () => {
      const { result } = renderHook(() => useP2PStore())

      // 事前にピアを追加
      const peer = {
        node_id: 'peer1',
        node_addr: '/ip4/192.168.1.1/tcp/4001',
        topics: ['topic1'],
        last_seen: Date.now(),
        connection_status: 'connected' as const,
      }

      act(() => {
        result.current.updatePeer(peer)
        result.current.removePeer('peer1')
      })

      expect(result.current.peers.has('peer1')).toBe(false)
    })
  })

  describe('clearError', () => {
    it('エラーをクリアできる', () => {
      const { result } = renderHook(() => useP2PStore())

      // エラーを設定
      act(() => {
        useP2PStore.setState({ error: 'Test error' })
      })

      expect(result.current.error).toBe('Test error')

      act(() => {
        result.current.clearError()
      })

      expect(result.current.error).toBe(null)
    })
  })

  describe('reset', () => {
    it('ストアを初期状態にリセットできる', () => {
      const { result } = renderHook(() => useP2PStore())

      // データを設定
      act(() => {
        const activeTopics = new Map()
        activeTopics.set('topic1', {
          topic_id: 'topic1',
          peer_count: 1,
          message_count: 1,
          recent_messages: [],
          connected_peers: [],
        })
        
        useP2PStore.setState({
          initialized: true,
          nodeId: 'node123',
          nodeAddr: '/ip4/127.0.0.1/tcp/4001',
          connectionStatus: 'connected',
          activeTopics,
        })
      })

      act(() => {
        result.current.reset()
      })

      expect(result.current.initialized).toBe(false)
      expect(result.current.nodeId).toBe(null)
      expect(result.current.nodeAddr).toBe(null)
      expect(result.current.connectionStatus).toBe('disconnected')
      expect(result.current.activeTopics.size).toBe(0)
    })
  })
})