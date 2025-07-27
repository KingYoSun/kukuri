import { useCallback, useEffect } from 'react';
import { useP2PStore, TopicStats, PeerInfo, P2PMessage } from '@/stores/p2pStore';
import { useP2PEventListener } from './useP2PEventListener';

// useP2Pフックの戻り値の型定義
export interface UseP2PReturn {
  // 状態
  initialized: boolean;
  nodeId: string | null;
  nodeAddr: string | null;
  activeTopics: TopicStats[];
  peers: PeerInfo[];
  connectionStatus: 'disconnected' | 'connecting' | 'connected' | 'error';
  error: string | null;

  // アクション
  joinTopic: (topicId: string, initialPeers?: string[]) => Promise<void>;
  leaveTopic: (topicId: string) => Promise<void>;
  broadcast: (topicId: string, content: string) => Promise<void>;
  clearError: () => void;

  // ヘルパー関数
  getTopicMessages: (topicId: string) => P2PMessage[];
  getTopicStats: (topicId: string) => TopicStats | undefined;
  isJoinedTopic: (topicId: string) => boolean;
  getConnectedPeerCount: () => number;
  getTopicPeerCount: (topicId: string) => number;
}

// P2P機能へのアクセスを提供するフック
export function useP2P(): UseP2PReturn {
  const {
    initialized,
    nodeId,
    nodeAddr,
    activeTopics,
    peers,
    messages,
    connectionStatus,
    error,
    initialize,
    joinTopic,
    leaveTopic,
    broadcast,
    refreshStatus,
    clearError,
  } = useP2PStore();

  // P2Pイベントリスナーを設定
  useP2PEventListener();

  // 初期化処理
  useEffect(() => {
    if (!initialized && connectionStatus === 'disconnected') {
      initialize();
    }
  }, [initialized, connectionStatus, initialize]);

  // 定期的な状態更新
  useEffect(() => {
    if (initialized && connectionStatus === 'connected') {
      const interval = setInterval(() => {
        refreshStatus();
      }, 30000); // 30秒ごとに更新

      return () => clearInterval(interval);
    }
  }, [initialized, connectionStatus, refreshStatus]);

  // トピックのメッセージを取得
  const getTopicMessages = useCallback(
    (topicId: string) => {
      return messages.get(topicId) || [];
    },
    [messages],
  );

  // トピックの統計情報を取得
  const getTopicStats = useCallback(
    (topicId: string) => {
      return activeTopics.get(topicId);
    },
    [activeTopics],
  );

  // トピックに参加しているかチェック
  const isJoinedTopic = useCallback(
    (topicId: string) => {
      return activeTopics.has(topicId);
    },
    [activeTopics],
  );

  // 接続中のピア数を取得
  const getConnectedPeerCount = useCallback(() => {
    return Array.from(peers.values()).filter((p) => p.connection_status === 'connected').length;
  }, [peers]);

  // トピックごとのピア数を取得
  const getTopicPeerCount = useCallback(
    (topicId: string) => {
      const stats = activeTopics.get(topicId);
      return stats?.peer_count || 0;
    },
    [activeTopics],
  );

  return {
    // 状態
    initialized,
    nodeId,
    nodeAddr,
    activeTopics: Array.from(activeTopics.values()),
    peers: Array.from(peers.values()),
    connectionStatus,
    error,

    // アクション
    joinTopic,
    leaveTopic,
    broadcast,
    clearError,

    // ヘルパー関数
    getTopicMessages,
    getTopicStats,
    isJoinedTopic,
    getConnectedPeerCount,
    getTopicPeerCount,
  };
}
