import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, fireEvent, waitFor } from '@testing-library/react'
import { TopicMeshVisualization } from '../TopicMeshVisualization'
import { useP2P } from '@/hooks/useP2P'

// useP2Pフックのモック
vi.mock('@/hooks/useP2P')

describe('TopicMeshVisualization', () => {
  const mockUseP2P = {
    getTopicStats: vi.fn(),
    getTopicMessages: vi.fn(),
    isJoinedTopic: vi.fn(),
    joinTopic: vi.fn(),
    leaveTopic: vi.fn(),
  }

  beforeEach(() => {
    vi.clearAllMocks()
    vi.mocked(useP2P).mockReturnValue(mockUseP2P as any)
    mockUseP2P.getTopicStats.mockReturnValue(null)
    mockUseP2P.getTopicMessages.mockReturnValue([])
    mockUseP2P.isJoinedTopic.mockReturnValue(false)
  })

  describe('未参加状態', () => {
    it('トピックに参加していない場合の表示', () => {
      render(<TopicMeshVisualization topicId="test-topic" />)

      expect(screen.getByText('トピックメッシュ')).toBeInTheDocument()
      expect(screen.getByText('P2Pネットワークの接続状況')).toBeInTheDocument()
      expect(screen.getByText('このトピックのP2Pネットワークに参加していません')).toBeInTheDocument()
      expect(screen.getByText('P2Pネットワークに参加')).toBeInTheDocument()
    })

    it('参加ボタンをクリックするとjoinTopicが呼ばれる', async () => {
      mockUseP2P.joinTopic.mockResolvedValueOnce(undefined)

      render(<TopicMeshVisualization topicId="test-topic" />)

      const joinButton = screen.getByText('P2Pネットワークに参加')
      fireEvent.click(joinButton)

      expect(screen.getByText('接続中...')).toBeInTheDocument()
      
      await waitFor(() => {
        expect(mockUseP2P.joinTopic).toHaveBeenCalledWith('test-topic')
      })
    })
  })

  describe('参加済み状態', () => {
    beforeEach(() => {
      mockUseP2P.isJoinedTopic.mockReturnValue(true)
    })

    it('統計情報が正しく表示される', () => {
      mockUseP2P.getTopicStats.mockReturnValue({
        topic_id: 'test-topic',
        peer_count: 5,
        message_count: 123,
        connected_peers: ['peer1', 'peer2', 'peer3'],
        recent_messages: [],
      })

      render(<TopicMeshVisualization topicId="test-topic" />)

      expect(screen.getByText('5')).toBeInTheDocument() // ピア数
      expect(screen.getByText('123')).toBeInTheDocument() // メッセージ数
    })

    it('接続中のピア一覧が表示される', () => {
      mockUseP2P.getTopicStats.mockReturnValue({
        topic_id: 'test-topic',
        peer_count: 3,
        message_count: 0,
        connected_peers: [
          'QmPeer1234567890abcdef',
          'QmPeer2234567890abcdef',
          'QmPeer3234567890abcdef',
        ],
        recent_messages: [],
      })

      render(<TopicMeshVisualization topicId="test-topic" />)

      expect(screen.getByText('接続中のピア')).toBeInTheDocument()
      expect(screen.getByText('QmPeer1234567890...')).toBeInTheDocument()
      expect(screen.getByText('QmPeer2234567890...')).toBeInTheDocument()
      expect(screen.getByText('QmPeer3234567890...')).toBeInTheDocument()
      expect(screen.getAllByText('接続中')).toHaveLength(3)
    })

    it('最近のメッセージが表示される', () => {
      const now = Date.now() / 1000
      mockUseP2P.getTopicMessages.mockReturnValue([
        {
          id: 'msg1',
          topic_id: 'test-topic',
          author: 'author1234567890',
          content: 'Hello P2P World!',
          timestamp: now,
          signature: 'sig1',
        },
        {
          id: 'msg2',
          topic_id: 'test-topic',
          author: 'author2234567890',
          content: 'This is a test message',
          timestamp: now - 60,
          signature: 'sig2',
        },
      ])

      render(<TopicMeshVisualization topicId="test-topic" />)

      expect(screen.getByText('最近のP2Pメッセージ')).toBeInTheDocument()
      expect(screen.getByText('author12...')).toBeInTheDocument()
      expect(screen.getByText('Hello P2P World!')).toBeInTheDocument()
      expect(screen.getByText('author22...')).toBeInTheDocument()
      expect(screen.getByText('This is a test message')).toBeInTheDocument()
    })

    it('切断ボタンをクリックするとleaveTopicが呼ばれる', async () => {
      mockUseP2P.leaveTopic.mockResolvedValueOnce(undefined)
      mockUseP2P.getTopicStats.mockReturnValue({
        topic_id: 'test-topic',
        peer_count: 1,
        message_count: 0,
        connected_peers: [],
        recent_messages: [],
      })

      render(<TopicMeshVisualization topicId="test-topic" />)

      const leaveButton = screen.getByText('切断')
      fireEvent.click(leaveButton)

      await waitFor(() => {
        expect(mockUseP2P.leaveTopic).toHaveBeenCalledWith('test-topic')
      })
    })

    it('空状態（ピアなし）の表示', () => {
      mockUseP2P.getTopicStats.mockReturnValue({
        topic_id: 'test-topic',
        peer_count: 0,
        message_count: 0,
        connected_peers: [],
        recent_messages: [],
      })

      render(<TopicMeshVisualization topicId="test-topic" />)

      expect(screen.getByText('まだピアに接続されていません')).toBeInTheDocument()
      expect(screen.getByText('他のノードがこのトピックに参加するのを待っています...')).toBeInTheDocument()
    })

    it('自動更新トグルが機能する', () => {
      mockUseP2P.getTopicStats.mockReturnValue({
        topic_id: 'test-topic',
        peer_count: 1,
        message_count: 0,
        connected_peers: ['peer1'],
        recent_messages: [],
      })

      render(<TopicMeshVisualization topicId="test-topic" />)

      // 自動更新ボタンを見つける（ActivityIconを含むボタン）
      const buttons = screen.getAllByRole('button')
      const autoRefreshButton = buttons.find(button => 
        button.querySelector('svg')?.classList.contains('text-green-500')
      )

      expect(autoRefreshButton).toBeTruthy()

      // クリックして自動更新を無効化
      fireEvent.click(autoRefreshButton!)

      // 再度クリックして有効化
      fireEvent.click(autoRefreshButton!)
    })
  })

  describe('メッセージ表示', () => {
    it('メッセージ数が10を超える場合は最新10件のみ表示', () => {
      mockUseP2P.isJoinedTopic.mockReturnValue(true)
      
      const messages = Array.from({ length: 15 }, (_, i) => ({
        id: `msg${i}`,
        topic_id: 'test-topic',
        author: `author${i}`,
        content: `Message ${i}`,
        timestamp: Date.now() / 1000 - i * 60,
        signature: `sig${i}`,
      }))

      mockUseP2P.getTopicMessages.mockReturnValue(messages)

      render(<TopicMeshVisualization topicId="test-topic" />)

      // 最初の10件のメッセージが表示されることを確認
      for (let i = 0; i < 10; i++) {
        expect(screen.getByText(`Message ${i}`)).toBeInTheDocument()
      }

      // 11件目以降は表示されない
      expect(screen.queryByText('Message 10')).not.toBeInTheDocument()
      expect(screen.queryByText('Message 14')).not.toBeInTheDocument()
    })

    it('タイムスタンプが正しくフォーマットされる', () => {
      mockUseP2P.isJoinedTopic.mockReturnValue(true)
      
      const timestamp = new Date('2024-01-01T12:34:56Z').getTime() / 1000
      mockUseP2P.getTopicMessages.mockReturnValue([{
        id: 'msg1',
        topic_id: 'test-topic',
        author: 'author1',
        content: 'Test message',
        timestamp,
        signature: 'sig1',
      }])

      render(<TopicMeshVisualization topicId="test-topic" />)

      // 日本時間での表示を確認（環境によって異なる可能性があるため、時刻形式の存在のみ確認）
      const timeElements = screen.getAllByText(/\d{1,2}:\d{2}:\d{2}/)
      expect(timeElements.length).toBeGreaterThan(0)
    })
  })
})