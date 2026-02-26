import { useEffect, useCallback, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';
import { useQueryClient } from '@tanstack/react-query';
import { useP2PStore, type P2PMessage, type PeerInfo } from '@/stores/p2pStore';
import { usePostStore } from '@/stores/postStore';
import { useTopicStore } from '@/stores/topicStore';
import { useUIStore } from '@/stores/uiStore';
import { errorHandler } from '@/lib/errorHandler';
import { validateNip01LiteMessage } from '@/lib/utils/nostrEventValidator';
import type { Post } from '@/stores/types';
import { applyKnownUserMetadata } from '@/lib/profile/userMetadata';
import { isTauriRuntime } from '@/lib/utils/tauriEnvironment';
import i18n from '@/i18n';
import { dispatchTimelineRealtimeDelta } from '@/lib/realtime/timelineRealtimeEvents';

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

interface P2PRawMessageEvent {
  topic_id: string;
  payload: unknown;
  timestamp: number;
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

const upsertPostIntoList = (posts: Post[] | undefined, post: Post): Post[] => {
  const filtered = (posts ?? []).filter((item) => item.id !== post.id);
  return [...filtered, post].sort((a, b) => b.created_at - a.created_at);
};

const textDecoder = new TextDecoder();
const RECENT_MESSAGE_ID_LIMIT = 2000;

const normalizeTimestampMillis = (value: unknown, fallbackSeconds: number): number => {
  if (typeof value === 'number' && Number.isFinite(value)) {
    return value > 1_000_000_000_000 ? Math.floor(value) : Math.floor(value * 1000);
  }
  if (typeof value === 'string') {
    const parsed = Date.parse(value);
    if (Number.isFinite(parsed)) {
      return parsed;
    }
  }
  return Math.floor(fallbackSeconds * 1000);
};

const parseRawPayload = (payload: unknown): Record<string, unknown> | null => {
  const parseJsonText = (value: string): Record<string, unknown> | null => {
    try {
      const parsed = JSON.parse(value);
      return parsed && typeof parsed === 'object' ? (parsed as Record<string, unknown>) : null;
    } catch {
      return null;
    }
  };

  if (payload instanceof Uint8Array) {
    try {
      return parseJsonText(textDecoder.decode(payload));
    } catch {
      return null;
    }
  }

  if (payload instanceof ArrayBuffer) {
    try {
      return parseJsonText(textDecoder.decode(new Uint8Array(payload)));
    } catch {
      return null;
    }
  }

  if (ArrayBuffer.isView(payload)) {
    try {
      return parseJsonText(
        textDecoder.decode(new Uint8Array(payload.buffer, payload.byteOffset, payload.byteLength)),
      );
    } catch {
      return null;
    }
  }

  if (payload && typeof payload === 'object' && !Array.isArray(payload)) {
    return payload as Record<string, unknown>;
  }

  if (typeof payload === 'string') {
    return parseJsonText(payload);
  }

  if (Array.isArray(payload) && payload.every((item) => typeof item === 'number')) {
    try {
      return parseJsonText(textDecoder.decode(Uint8Array.from(payload)));
    } catch {
      return null;
    }
  }

  return null;
};

const parseRawEventMessage = (
  topicId: string,
  payload: unknown,
  fallbackSeconds: number,
): P2PMessage | null => {
  const parsed = parseRawPayload(payload);
  if (!parsed) {
    return null;
  }

  const id = parsed.id;
  const content = parsed.content;
  const author = typeof parsed.author === 'string' ? parsed.author : parsed.pubkey;
  const signature = typeof parsed.signature === 'string' ? parsed.signature : parsed.sig;

  if (
    typeof id !== 'string' ||
    typeof author !== 'string' ||
    typeof content !== 'string' ||
    typeof signature !== 'string'
  ) {
    return null;
  }

  return {
    id,
    topic_id: topicId,
    author,
    content,
    timestamp: normalizeTimestampMillis(parsed.created_at ?? parsed.timestamp, fallbackSeconds),
    signature,
  };
};

export function useP2PEventListener() {
  const queryClient = useQueryClient();
  const { addMessage, updatePeer, removePeer, refreshStatus } = useP2PStore();
  const { addPost } = usePostStore();
  const { updateTopicPostCount } = useTopicStore();
  const recentMessageIds = useRef<Set<string>>(new Set());
  const recentMessageOrder = useRef<string[]>([]);

  const shouldHandleMessage = useCallback((messageId: string): boolean => {
    if (recentMessageIds.current.has(messageId)) {
      return false;
    }

    recentMessageIds.current.add(messageId);
    recentMessageOrder.current.push(messageId);

    if (recentMessageOrder.current.length > RECENT_MESSAGE_ID_LIMIT) {
      const removed = recentMessageOrder.current.shift();
      if (removed) {
        recentMessageIds.current.delete(removed);
      }
    }

    return true;
  }, []);

  const handleP2PMessageAsPost = useCallback(
    (message: P2PMessage, topicId: string) => {
      try {
        const author = applyKnownUserMetadata({
          id: message.author,
          pubkey: message.author,
          npub: message.author,
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
        queryClient.setQueryData<Post[]>(['timeline'], (prev) => upsertPostIntoList(prev, post));
        queryClient.setQueryData<Post[]>(['posts', 'all'], (prev) =>
          upsertPostIntoList(prev, post),
        );
        queryClient.setQueryData<Post[]>(['posts', topicId], (prev) =>
          upsertPostIntoList(prev, post),
        );
        const timelineUpdateMode = useUIStore.getState().timelineUpdateMode;
        if (timelineUpdateMode === 'standard') {
          queryClient.invalidateQueries({ queryKey: ['topicTimeline', topicId] });
          queryClient.invalidateQueries({ queryKey: ['topicThreads', topicId] });
          queryClient.invalidateQueries({ queryKey: ['threadPosts', topicId] });
        } else {
          dispatchTimelineRealtimeDelta({
            source: 'p2p',
            topicId,
            message,
          });
        }
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

  const handleIncomingP2PMessage = useCallback(
    (topic_id: string, message: P2PMessage, context: string) => {
      const v = validateNip01LiteMessage(message);
      if (!v.ok) {
        errorHandler.log('Drop invalid P2P message (NIP-01 lite)', v.reason, {
          context,
          showToast: false,
        });
        return;
      }

      if (!shouldHandleMessage(message.id)) {
        return;
      }

      const p2pMessage: P2PMessage = {
        ...message,
        topic_id,
      };

      addMessage(p2pMessage);

      const messageTimestampSeconds =
        p2pMessage.timestamp > 1_000_000_000_000
          ? Math.floor(p2pMessage.timestamp / 1000)
          : Math.floor(p2pMessage.timestamp);

      useTopicStore.getState().handleIncomingTopicMessage(topic_id, messageTimestampSeconds);
      handleP2PMessageAsPost(p2pMessage, topic_id);
      window.dispatchEvent(new Event('realtime-update'));
    },
    [addMessage, handleP2PMessageAsPost, shouldHandleMessage],
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
        handleIncomingP2PMessage(
          topic_id,
          { ...message, topic_id },
          'useP2PEventListener.p2p://message',
        );
      },
      'useP2PEventListener.p2p://message',
    );

    void registerListener<P2PRawMessageEvent>(
      'p2p://message/raw',
      ({ topic_id, payload, timestamp }) => {
        const parsed = parseRawEventMessage(topic_id, payload, timestamp);
        if (!parsed) {
          return;
        }
        handleIncomingP2PMessage(topic_id, parsed, 'useP2PEventListener.p2p://message/raw');
      },
      'useP2PEventListener.p2p://message/raw',
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
  }, [updatePeer, removePeer, refreshStatus, handleIncomingP2PMessage]);
}
