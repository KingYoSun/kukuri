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
import { applyKnownUserMetadata } from '@/lib/profile/userMetadata';
import { isTauriRuntime } from '@/lib/utils/tauriEnvironment';
import i18n from '@/i18n';

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

export function useP2PEventListener() {
  const queryClient = useQueryClient();
  const { addMessage, updatePeer, removePeer, refreshStatus } = useP2PStore();
  const { addPost } = usePostStore();
  const { updateTopicPostCount } = useTopicStore();

  const handleP2PMessageAsPost = useCallback(
    async (message: P2PMessage, topicId: string) => {
      try {
        const author = applyKnownUserMetadata({
          id: message.author,
          pubkey: message.author,
          npub: await pubkeyToNpub(message.author),
          name: i18n.t('p2p.unknownUser'),
          displayName: i18n.t('p2p.unknownUser'),
          about: '',
          picture: '',
          nip05: '',
          avatar: null,
          publicProfile: true,
          showOnlineStatus: false,
        });

        const post: Post = {
          id: message.id,
          content: message.content,
          author: author,
          topicId,
          created_at: Math.floor(message.timestamp / 1000),
          tags: [],
          likes: 0,
          boosts: 0,
          replies: [],
        };

        addPost(post);
        queryClient.invalidateQueries({ queryKey: ['posts', topicId] });
        queryClient.invalidateQueries({ queryKey: ['posts'] });
        queryClient.invalidateQueries({ queryKey: ['topicTimeline', topicId] });
        queryClient.invalidateQueries({ queryKey: ['topicThreads', topicId] });
        queryClient.invalidateQueries({ queryKey: ['threadPosts', topicId] });
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
    if (!isTauriRuntime()) {
      return;
    }

    const unlisteners: Array<() => void> = [];

    const registerListener = async <T>(
      event: string,
      handler: (payload: T) => void,
      context: string,
    ) => {
      try {
        const unlisten = await listen<T>(event, (evt) => handler(evt.payload));
        unlisteners.push(() => {
          try {
            unlisten();
          } catch (error) {
            errorHandler.log('P2P event unlisten failed', error, { context });
          }
        });
      } catch (error) {
        errorHandler.log('P2P event subscription failed', error, {
          context,
          metadata: { event },
        });
      }
    };

    void registerListener<P2PMessageEvent>(
      'p2p://message',
      ({ topic_id, message }) => {
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

        addMessage(p2pMessage);

        const messageTimestampSeconds =
          message.timestamp > 1_000_000_000_000
            ? Math.floor(message.timestamp / 1000)
            : Math.floor(message.timestamp);

        useTopicStore.getState().handleIncomingTopicMessage(topic_id, messageTimestampSeconds);
        handleP2PMessageAsPost(p2pMessage, topic_id);
        window.dispatchEvent(new Event('realtime-update'));
      },
      'useP2PEventListener.p2p://message',
    );

    void registerListener<P2PPeerEvent>(
      'p2p://peer',
      ({ topic_id, peer_id, event_type }) => {
        if (event_type === 'joined') {
          const peerInfo: PeerInfo = {
            node_id: peer_id,
            node_addr: '',
            topics: [topic_id],
            last_seen: Date.now(),
            connection_status: 'connected',
          };
          updatePeer(peerInfo);
        } else if (event_type === 'left') {
          removePeer(peer_id);
        }

        refreshStatus();
      },
      'useP2PEventListener.p2p://peer',
    );

    void registerListener<P2PConnectionEvent>(
      'p2p://connection',
      ({ node_id, node_addr, status }) => {
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
      },
      'useP2PEventListener.p2p://connection',
    );

    void registerListener<{ error: string }>(
      'p2p://error',
      ({ error }) => {
        errorHandler.log('P2P error', error, {
          context: 'useP2PEventListener',
          showToast: true,
          toastTitle: i18n.t('p2p.networkError'),
        });
        useP2PStore.getState().clearError();
      },
      'useP2PEventListener.p2p://error',
    );

    return () => {
      unlisteners.forEach((unlisten) => {
        try {
          unlisten();
        } catch (error) {
          errorHandler.log('P2P event cleanup failed', error, {
            context: 'useP2PEventListener.cleanup',
          });
        }
      });
    };
  }, [addMessage, updatePeer, removePeer, refreshStatus, handleP2PMessageAsPost]);
}
