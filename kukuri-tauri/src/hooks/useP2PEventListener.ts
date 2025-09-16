import { useEffect, useCallback } from 'react';
import { listen } from '@tauri-apps/api/event';
import { useQueryClient } from '@tanstack/react-query';
import { useP2PStore, type P2PMessage, type PeerInfo } from '@/stores/p2pStore';
import { usePostStore } from '@/stores/postStore';
import { useTopicStore } from '@/stores/topicStore';
import { errorHandler } from '@/lib/errorHandler';
import { validateNip01LiteMessage } from '@/lib/utils/nostrEventValidator';
import type { Post } from '@/stores/types';
import { pubkeyToNpub } from '@/lib/utils/nostr';

// P2Pイベントの型定義
interface P2PMessageEvent {
  topic_id: string;
  message: {
    id: string;
    author: string;
    content: string;
    timestamp: number;
    signature: string;
  };
}

interface P2PPeerEvent {
  topic_id: string;
  peer_id: string;
  event_type: 'joined' | 'left';
}

interface P2PConnectionEvent {
  node_id: string;
  node_addr: string;
  status: 'connected' | 'disconnected';
}

// P2Pイベントリスナーフック
export function useP2PEventListener() {
  const queryClient = useQueryClient();
  const { addMessage, updatePeer, removePeer, refreshStatus } = useP2PStore();
  const { addPost } = usePostStore();
  const { updateTopicPostCount } = useTopicStore();

  // P2Pメッセージを投稿として処理
  const handleP2PMessageAsPost = useCallback(
    async (message: P2PMessage, topicId: string) => {
      try {
        // P2Pメッセージを投稿形式に変換
        const post: Post = {
          id: message.id,
          content: message.content,
          author: {
            id: message.author,
            pubkey: message.author,
            npub: await pubkeyToNpub(message.author),
            name: 'P2Pユーザー',
            displayName: 'P2Pユーザー',
            about: '',
            picture: '',
            nip05: '',
          },
          topicId,
          created_at: Math.floor(message.timestamp / 1000), // ミリ秒から秒に変換
          tags: [],
          likes: 0,
          boosts: 0,
          replies: [],
        };

        // ストアに追加
        addPost(post);

        // React Queryのキャッシュを無効化して最新データを反映
        queryClient.invalidateQueries({ queryKey: ['posts', topicId] });
        queryClient.invalidateQueries({ queryKey: ['posts'] });

        // トピックの投稿数を更新
        updateTopicPostCount(topicId, 1);
      } catch (error) {
        errorHandler.log('Failed to process P2P message as post', error, {
          context: 'useP2PEventListener.handleP2PMessageAsPost',
          showToast: false,
        });
      }
    },
    [addPost, queryClient, updateTopicPostCount],
  );

  useEffect(() => {
    const unlisteners: Promise<() => void>[] = [];

    // P2Pメッセージ受信
    unlisteners.push(
      listen<P2PMessageEvent>('p2p://message', (event) => {
        const { topic_id, message } = event.payload;
        // 最小NIP-01形状の検証（不正は破棄）
        const v = validateNip01LiteMessage(message);
        if (!v.ok) {
          errorHandler.log('Drop invalid P2P message (NIP-01 lite)', v.reason, {
            context: 'useP2PEventListener.p2p://message',
            showToast: false,
          });
          return;
        }

        const p2pMessage: P2PMessage = {
          ...message,
          topic_id,
        };

        // P2Pストアに追加
        addMessage(p2pMessage);

        // 投稿として処理（リアルタイム更新）
        handleP2PMessageAsPost(p2pMessage, topic_id);

        // リアルタイム更新イベントを発火
        window.dispatchEvent(new Event('realtime-update'));
      }),
    );

    // ピアイベント（参加/離脱）
    unlisteners.push(
      listen<P2PPeerEvent>('p2p://peer', (event) => {
        const { topic_id, peer_id, event_type } = event.payload;

        if (event_type === 'joined') {
          // ピア参加時の処理
          const peerInfo: PeerInfo = {
            node_id: peer_id,
            node_addr: '', // 後で実際のアドレスを取得
            topics: [topic_id],
            last_seen: Date.now(),
            connection_status: 'connected',
          };
          updatePeer(peerInfo);
        } else if (event_type === 'left') {
          // ピア離脱時の処理
          removePeer(peer_id);
        }

        // 状態を更新
        refreshStatus();
      }),
    );

    // 接続状態イベント
    unlisteners.push(
      listen<P2PConnectionEvent>('p2p://connection', (event) => {
        const { node_id, node_addr, status } = event.payload;

        if (status === 'connected') {
          const peerInfo: PeerInfo = {
            node_id,
            node_addr,
            topics: [],
            last_seen: Date.now(),
            connection_status: 'connected',
          };
          updatePeer(peerInfo);
        } else {
          removePeer(node_id);
        }
      }),
    );

    // エラーイベント
    unlisteners.push(
      listen<{ error: string }>('p2p://error', (event) => {
        errorHandler.log('P2P error', event.payload.error, {
          context: 'useP2PEventListener',
          showToast: true,
          toastTitle: 'P2Pネットワークエラー',
        });
        useP2PStore.getState().clearError();
      }),
    );
    // クリーンアップ
    return () => {
      unlisteners.forEach(async (unlisten) => {
        const fn = await unlisten;
        fn();
      });
    };
  }, [addMessage, updatePeer, removePeer, refreshStatus, handleP2PMessageAsPost]);
}
