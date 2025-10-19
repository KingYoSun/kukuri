import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { P2PDebugPanel } from '../P2PDebugPanel';
import { useP2P, UseP2PReturn } from '@/hooks/useP2P';
import { useNostrSubscriptions } from '@/hooks/useNostrSubscriptions';

// useP2Pフックのモック
vi.mock('@/hooks/useP2P');
vi.mock('@/hooks/useNostrSubscriptions');

describe('P2PDebugPanel', () => {
  const originalEnv = { ...import.meta.env };

  beforeEach(() => {
    // テスト環境ではimport.meta.env.PRODをfalseに設定
    Object.defineProperty(import.meta, 'env', {
      value: { ...originalEnv, PROD: false, MODE: 'test' },
      writable: true,
    });
  });

  afterEach(() => {
    Object.defineProperty(import.meta, 'env', {
      value: originalEnv,
      writable: true,
    });
  });
  const createMockUseP2P = (): UseP2PReturn => ({
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
  });

  const createMockSubscriptions = () => ({
    subscriptions: [],
    isLoading: false,
    error: null,
    refresh: vi.fn(),
  });

  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useP2P).mockReturnValue(createMockUseP2P());
    vi.mocked(useNostrSubscriptions).mockReturnValue(createMockSubscriptions());
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
      const mockData = createMockUseP2P();
      mockData.peers = [
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
      ];
      mockData.activeTopics = [
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
      ];
      vi.mocked(useP2P).mockReturnValue(mockData);

      render(<P2PDebugPanel />);

      // ピア数を確認
      expect(screen.getByText('接続ピア数')).toBeInTheDocument();
      const peerCountBadges = screen.getAllByText('2');
      expect(peerCountBadges.length).toBeGreaterThan(0);

      // トピック数を確認
      expect(screen.getByText('参加トピック数')).toBeInTheDocument();
    });

    it('エラーが表示され、クリアできる', async () => {
      const mockData = createMockUseP2P();
      mockData.error = 'テストエラーメッセージ';
      vi.mocked(useP2P).mockReturnValue(mockData);

      const user = userEvent.setup();
      render(<P2PDebugPanel />);

      expect(screen.getByText('テストエラーメッセージ')).toBeInTheDocument();

      const clearButton = screen.getByText('エラーをクリア');
      await user.click(clearButton);

      expect(mockData.clearError).toHaveBeenCalledTimes(1);
    });
  });

  describe('トピックタブ', () => {
    it('新しいトピックに参加できる', async () => {
      const mockData = createMockUseP2P();
      vi.mocked(useP2P).mockReturnValue(mockData);

      const user = userEvent.setup();
      render(<P2PDebugPanel />);

      // トピックタブに切り替え
      const topicsTab = screen.getByText('トピック');
      await user.click(topicsTab);

      // タブコンテンツが表示されることを確認
      await waitFor(() => {
        const input = screen.getByPlaceholderText('トピックID (例: test-topic)');
        expect(input).toBeInTheDocument();
      });

      const input = screen.getByPlaceholderText('トピックID (例: test-topic)');
      await user.type(input, 'new-topic');

      const joinButton = screen.getByRole('button', { name: '参加' });
      await user.click(joinButton);

      await waitFor(() => {
        expect(mockData.joinTopic).toHaveBeenCalledWith('new-topic');
      });
    });

    it('参加中のトピック一覧が表示される', async () => {
      const mockData = createMockUseP2P();
      mockData.activeTopics = [
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
      ];
      vi.mocked(useP2P).mockReturnValue(mockData);

      const user = userEvent.setup();
      render(<P2PDebugPanel />);

      const topicsTab = screen.getByText('トピック');
      await user.click(topicsTab);

      // タブコンテンツが表示されるまで待つ
      await waitFor(
        () => {
          // ピア数とメッセージ数を確認
          expect(screen.getByText(/ピア: 5/)).toBeInTheDocument();
          expect(screen.getByText(/メッセージ: 100/)).toBeInTheDocument();
          expect(screen.getByText(/ピア: 3/)).toBeInTheDocument();
          expect(screen.getByText(/メッセージ: 50/)).toBeInTheDocument();
        },
        { timeout: 3000 },
      );
    });

    it('トピックから離脱できる', async () => {
      const mockData = createMockUseP2P();
      mockData.activeTopics = [
        {
          topic_id: 'topic1',
          peer_count: 1,
          message_count: 0,
          recent_messages: [],
          connected_peers: [],
        },
      ];
      vi.mocked(useP2P).mockReturnValue(mockData);

      const user = userEvent.setup();
      render(<P2PDebugPanel />);

      const topicsTab = screen.getByText('トピック');
      await user.click(topicsTab);

      // タブコンテンツが表示されるまで待つ
      await waitFor(
        () => {
          expect(screen.getByText(/ピア: 1/)).toBeInTheDocument();
        },
        { timeout: 3000 },
      );

      // 削除ボタンをクリック（TrashIconを含むボタン）
      const deleteButtons = screen
        .getAllByRole('button')
        .filter((button) => button.querySelector('svg'));
      const deleteButton = deleteButtons[deleteButtons.length - 1];
      await user.click(deleteButton);

      await waitFor(() => {
        expect(mockData.leaveTopic).toHaveBeenCalledWith('topic1');
      });
    });
  });

  describe('送信タブ', () => {
    it('トピックが選択されていない場合のメッセージ', async () => {
      const user = userEvent.setup();
      render(<P2PDebugPanel />);

      const broadcastTab = screen.getByText('送信');
      await user.click(broadcastTab);

      await waitFor(() => {
        expect(screen.getByText('トピックを選択してください')).toBeInTheDocument();
      });
    });

    it('メッセージをブロードキャストできる', async () => {
      const mockData = createMockUseP2P();
      mockData.activeTopics = [
        {
          topic_id: 'topic1',
          peer_count: 2,
          message_count: 10,
          recent_messages: [],
          connected_peers: [],
        },
      ];
      vi.mocked(useP2P).mockReturnValue(mockData);

      const user = userEvent.setup();
      render(<P2PDebugPanel />);

      // トピックタブでトピックを選択
      const topicsTab = screen.getByText('トピック');
      await user.click(topicsTab);

      await waitFor(() => {
        expect(screen.getByRole('button', { name: '選択' })).toBeInTheDocument();
      });

      const selectButton = screen.getByRole('button', { name: '選択' });
      await user.click(selectButton);

      // 送信タブに切り替え
      const broadcastTab = screen.getByText('送信');
      await user.click(broadcastTab);

      await waitFor(() => {
        expect(screen.getByText('topic1')).toBeInTheDocument();
      });

      const messageInput = screen.getByPlaceholderText('送信するメッセージを入力');
      await user.type(messageInput, 'Hello P2P!');

      const sendButton = screen.getByRole('button', { name: 'ブロードキャスト' });
      await user.click(sendButton);

      await waitFor(() => {
        expect(mockData.broadcast).toHaveBeenCalledWith('topic1', 'Hello P2P!');
      });
    });
  });

  describe('ログタブ', () => {
    it('初期状態ではログが空', async () => {
      const user = userEvent.setup();
      render(<P2PDebugPanel />);

      const logsTab = screen.getByText('ログ');
      await user.click(logsTab);

      await waitFor(() => {
        expect(screen.getByText('ログはありません')).toBeInTheDocument();
      });
    });

    it('操作ログが記録される', async () => {
      const user = userEvent.setup();
      render(<P2PDebugPanel />);

      // トピックに参加
      const topicsTab = screen.getByText('トピック');
      await user.click(topicsTab);

      await waitFor(() => {
        expect(screen.getByRole('textbox')).toBeInTheDocument();
      });

      const input = screen.getByRole('textbox');
      await user.type(input, 'log-test');

      const joinButton = screen.getByRole('button', { name: '参加' });
      await user.click(joinButton);

      // ログタブに切り替え
      const logsTab = screen.getByText('ログ');
      await user.click(logsTab);

      await waitFor(() => {
        expect(screen.getByText(/Joining topic: log-test/)).toBeInTheDocument();
      });
    });

    it('ログをクリアできる', async () => {
      const user = userEvent.setup();
      render(<P2PDebugPanel />);

      // 何か操作してログを生成
      const topicsTab = screen.getByText('トピック');
      await user.click(topicsTab);

      await waitFor(() => {
        expect(screen.getByRole('textbox')).toBeInTheDocument();
      });

      const input = screen.getByRole('textbox');
      await user.type(input, 'test');
      await user.click(screen.getByRole('button', { name: '参加' }));

      // ログタブに切り替え
      const logsTab = screen.getByText('ログ');
      await user.click(logsTab);

      await waitFor(() => {
        expect(screen.queryByText('ログはありません')).not.toBeInTheDocument();
      });

      const clearButton = screen.getByRole('button', { name: 'クリア' });
      await user.click(clearButton);

      await waitFor(() => {
        expect(screen.getByText('ログはありません')).toBeInTheDocument();
      });
    });
  });
});
