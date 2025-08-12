import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { createLocalStoragePersist } from './utils/persistHelpers';
import { p2pApi } from '@/lib/api/p2p';
import { errorHandler } from '@/lib/errorHandler';

// P2Pメッセージ
export interface P2PMessage {
  id: string;
  topic_id: string;
  author: string;
  content: string;
  timestamp: number;
  signature: string;
}

// トピックメッシュの統計情報
export interface TopicStats {
  topic_id: string;
  peer_count: number;
  message_count: number;
  recent_messages: P2PMessage[];
  connected_peers: string[];
}

// ピア情報
export interface PeerInfo {
  node_id: string;
  node_addr: string;
  topics: string[];
  last_seen: number;
  connection_status: 'connected' | 'disconnected' | 'connecting';
}

// P2Pストア
interface P2PStore {
  // 状態
  initialized: boolean;
  nodeId: string | null;
  nodeAddr: string | null;
  activeTopics: Map<string, TopicStats>;
  peers: Map<string, PeerInfo>;
  messages: Map<string, P2PMessage[]>; // topic_id -> messages
  connectionStatus: 'disconnected' | 'connecting' | 'connected' | 'error';
  error: string | null;

  // アクション
  initialize: () => Promise<void>;
  joinTopic: (topicId: string, initialPeers?: string[]) => Promise<void>;
  leaveTopic: (topicId: string) => Promise<void>;
  broadcast: (topicId: string, content: string) => Promise<void>;
  refreshStatus: () => Promise<void>;
  addMessage: (message: P2PMessage) => void;
  updatePeer: (peer: PeerInfo) => void;
  removePeer: (nodeId: string) => void;
  clearError: () => void;
  reset: () => void;
}

export const useP2PStore = create<P2PStore>()(
  persist(
    (set, get) => ({
      // 初期状態
      initialized: false,
      nodeId: null,
      nodeAddr: null,
      activeTopics: new Map(),
      peers: new Map(),
      messages: new Map(),
      connectionStatus: 'disconnected',
      error: null,

      // P2P初期化
      initialize: async () => {
        try {
          set({ connectionStatus: 'connecting', error: null });

          // P2P機能を初期化
          await p2pApi.initialize();

          // ノード情報を取得
          const nodeAddr = await p2pApi.getNodeAddress();

          // P2P状態を取得
          const status = await p2pApi.getStatus();

          set({
            initialized: true,
            nodeAddr: nodeAddr ? nodeAddr.join(', ') : '',
            nodeId: status?.endpoint_id || '',
            connectionStatus: 'connected',
          });
        } catch (error) {
          errorHandler.log('Failed to initialize P2P', error, {
            context: 'P2PStore.initialize',
            showToast: true,
            toastTitle: 'P2P接続に失敗しました',
          });
          set({
            connectionStatus: 'error',
            error: error instanceof Error ? error.message : 'P2P initialization failed',
          });
        }
      },

      // トピック参加
      joinTopic: async (topicId: string, initialPeers: string[] = []) => {
        try {
          await p2pApi.joinTopic(topicId, initialPeers);

          // トピック統計情報を初期化
          const activeTopics = new Map(get().activeTopics);
          activeTopics.set(topicId, {
            topic_id: topicId,
            peer_count: 0,
            message_count: 0,
            recent_messages: [],
            connected_peers: [],
          });

          set({ activeTopics });

          // 状態を更新
          await get().refreshStatus();
        } catch (error) {
          errorHandler.log('Failed to join topic', error, {
            context: 'P2PStore.joinTopic',
            showToast: true,
            toastTitle: 'トピックへの参加に失敗しました',
          });
          set({
            error: error instanceof Error ? error.message : 'Failed to join topic',
          });
        }
      },

      // トピック離脱
      leaveTopic: async (topicId: string) => {
        try {
          await p2pApi.leaveTopic(topicId);

          // トピック情報を削除
          const activeTopics = new Map(get().activeTopics);
          activeTopics.delete(topicId);

          // メッセージも削除
          const messages = new Map(get().messages);
          messages.delete(topicId);

          set({ activeTopics, messages });
        } catch (error) {
          errorHandler.log('Failed to leave topic', error, {
            context: 'P2PStore.leaveTopic',
            showToast: true,
            toastTitle: 'トピックからの離脱に失敗しました',
          });
          set({
            error: error instanceof Error ? error.message : 'Failed to leave topic',
          });
        }
      },

      // メッセージブロードキャスト
      broadcast: async (topicId: string, content: string) => {
        try {
          await p2pApi.broadcast(topicId, content);
        } catch (error) {
          errorHandler.log('Failed to broadcast message', error, {
            context: 'P2PStore.broadcast',
            showToast: true,
            toastTitle: 'メッセージの送信に失敗しました',
          });
          set({
            error: error instanceof Error ? error.message : 'Failed to broadcast message',
          });
        }
      },

      // 状態更新
      refreshStatus: async () => {
        try {
          const status = await p2pApi.getStatus();

          // アクティブトピックの統計情報を更新
          const activeTopics = new Map<string, TopicStats>();

          for (const stats of status.active_topics) {
            const currentStats = get().activeTopics.get(stats.topic_id) || {
              topic_id: stats.topic_id,
              peer_count: 0,
              message_count: 0,
              recent_messages: [],
              connected_peers: [],
            };

            activeTopics.set(stats.topic_id, {
              ...currentStats,
              peer_count: stats.peer_count,
              message_count: stats.message_count,
            });
          }

          set({ activeTopics });
        } catch (error) {
          errorHandler.log('Failed to refresh P2P status', error, {
            context: 'P2PStore.refreshStatus',
          });
        }
      },

      // メッセージ追加
      addMessage: (message: P2PMessage) => {
        const messages = new Map(get().messages);
        const topicMessages = messages.get(message.topic_id) || [];

        // 重複チェック
        if (!topicMessages.find((m) => m.id === message.id)) {
          // メッセージを追加（最新のものを先頭に）
          messages.set(message.topic_id, [message, ...topicMessages].slice(0, 100));

          // トピック統計も更新
          const activeTopics = new Map(get().activeTopics);
          const topicStats = activeTopics.get(message.topic_id);

          if (topicStats) {
            activeTopics.set(message.topic_id, {
              ...topicStats,
              message_count: topicStats.message_count + 1,
              recent_messages: [message, ...topicStats.recent_messages].slice(0, 10),
            });
          }

          set({ messages, activeTopics });
        }
      },

      // ピア情報更新
      updatePeer: (peer: PeerInfo) => {
        const peers = new Map(get().peers);
        peers.set(peer.node_id, peer);
        set({ peers });
      },

      // ピア削除
      removePeer: (nodeId: string) => {
        const peers = new Map(get().peers);
        peers.delete(nodeId);
        set({ peers });
      },

      // エラークリア
      clearError: () => {
        set({ error: null });
      },

      // リセット
      reset: () => {
        set({
          initialized: false,
          nodeId: null,
          nodeAddr: null,
          activeTopics: new Map(),
          peers: new Map(),
          messages: new Map(),
          connectionStatus: 'disconnected',
          error: null,
        });
      },
    }),
    createLocalStoragePersist(
      'p2p-storage',
      (state) => ({
        // 永続化する状態を選択（Mapは除外）
        initialized: state.initialized,
        nodeId: state.nodeId,
        nodeAddr: state.nodeAddr,
      }),
    ),
  ),
);
