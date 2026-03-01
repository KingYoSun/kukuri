import { useEffect, useCallback, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';
import { useQueryClient } from '@tanstack/react-query';
import { useP2PStore, type P2PMessage, type PeerInfo } from '@/stores/p2pStore';
import { usePostStore } from '@/stores/postStore';
import { useTopicStore } from '@/stores/topicStore';
import { useUIStore } from '@/stores/uiStore';
import { errorHandler } from '@/lib/errorHandler';
import { validateNip01LiteMessage } from '@/lib/utils/nostrEventValidator';
import type { Post, User } from '@/stores/types';
import { applyKnownUserMetadata } from '@/lib/profile/userMetadata';
import { isTauriRuntime } from '@/lib/utils/tauriEnvironment';
import { isHexFormat, pubkeyToNpub } from '@/lib/utils/nostr';
import { NostrEventKind } from '@/types/nostr';
import i18n from '@/i18n';
import { dispatchTimelineRealtimeDelta } from '@/lib/realtime/timelineRealtimeEvents';
import { TauriApi } from '@/lib/api/tauri';
import { mapUserProfileToUser } from '@/lib/profile/profileMapper';
import type { TopicTimelineEntry } from './usePosts';

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

const sortTimelineEntries = (entries: TopicTimelineEntry[]): TopicTimelineEntry[] =>
  [...entries].sort((a, b) => b.lastActivityAt - a.lastActivityAt);

const upsertTimelineEntry = (
  entries: TopicTimelineEntry[] | undefined,
  post: Post,
  threadUuid: string,
  isReply: boolean,
): TopicTimelineEntry[] => {
  const base = entries ?? [];
  const existingIndex = base.findIndex((entry) => entry.threadUuid === threadUuid);
  if (existingIndex >= 0) {
    const next = [...base];
    const existing = next[existingIndex];
    if (isReply) {
      const isSameFirstReply = existing.firstReply?.id === post.id;
      next[existingIndex] = {
        ...existing,
        firstReply: isSameFirstReply ? post : (existing.firstReply ?? post),
        replyCount: isSameFirstReply ? existing.replyCount : Math.max(existing.replyCount + 1, 1),
        lastActivityAt: Math.max(existing.lastActivityAt, post.created_at),
      };
    } else {
      next[existingIndex] = {
        ...existing,
        parentPost: existing.parentPost.id === post.id ? post : existing.parentPost,
        lastActivityAt: Math.max(existing.lastActivityAt, post.created_at),
      };
    }
    return sortTimelineEntries(next);
  }

  if (isReply) {
    return base;
  }

  return sortTimelineEntries([
    {
      threadUuid,
      parentPost: post,
      firstReply: null,
      replyCount: 0,
      lastActivityAt: post.created_at,
    },
    ...base,
  ]);
};

const textDecoder = new TextDecoder();
const RECENT_MESSAGE_ID_LIMIT = 2000;
const AUTHOR_PROFILE_MISS_TTL_MS = 60_000;
const THREAD_PATH_SEGMENT = '/threads/';

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

const shortenIdentifier = (value: string): string => {
  const trimmed = value.trim();
  if (!trimmed) {
    return i18n.t('p2p.unknownUser');
  }
  if (trimmed.length <= 16) {
    return trimmed;
  }
  return `${trimmed.slice(0, 8)}...${trimmed.slice(-4)}`;
};

const normalizeMessageTags = (value: unknown): string[][] => {
  if (!Array.isArray(value)) {
    return [];
  }
  return value
    .filter(
      (tag): tag is string[] => Array.isArray(tag) && tag.every((item) => typeof item === 'string'),
    )
    .map((tag) => [...tag]);
};

const deriveThreadUuidFromEventId = (eventId: string): string => {
  const normalizedHex = eventId
    .toLowerCase()
    .replace(/[^0-9a-f]/g, '')
    .padEnd(32, '0')
    .slice(0, 32);
  const bytes = new Uint8Array(16);
  for (let index = 0; index < 16; index += 1) {
    const byteHex = normalizedHex.slice(index * 2, index * 2 + 2);
    const parsed = Number.parseInt(byteHex, 16);
    bytes[index] = Number.isNaN(parsed) ? 0 : parsed;
  }

  // RFC 4122 variant + version(5) bits to keep UUID tooling compatible.
  bytes[6] = (bytes[6] & 0x0f) | 0x50;
  bytes[8] = (bytes[8] & 0x3f) | 0x80;

  const toHex = (value: number): string => value.toString(16).padStart(2, '0');
  const digest = Array.from(bytes, toHex).join('');
  return `${digest.slice(0, 8)}-${digest.slice(8, 12)}-${digest.slice(12, 16)}-${digest.slice(16, 20)}-${digest.slice(20, 32)}`;
};

const findTagValue = (tags: string[][], key: string): string | null => {
  const value = tags.find((tag) => tag[0] === key)?.[1];
  const trimmed = value?.trim();
  return trimmed ? trimmed : null;
};

const extractThreadUuidFromTags = (tags: string[][], topicId: string): string | null => {
  const explicitThreadUuid = findTagValue(tags, 'thread_uuid');
  if (explicitThreadUuid) {
    return explicitThreadUuid;
  }

  const threadNamespace = findTagValue(tags, 'thread');
  if (!threadNamespace) {
    return null;
  }

  const topicScopedPrefix = `${topicId}${THREAD_PATH_SEGMENT}`;
  const topicScopedIndex = threadNamespace.indexOf(topicScopedPrefix);
  if (topicScopedIndex >= 0) {
    const uuid = threadNamespace.slice(topicScopedIndex + topicScopedPrefix.length).trim();
    return uuid || null;
  }

  const lastSegmentIndex = threadNamespace.lastIndexOf(THREAD_PATH_SEGMENT);
  if (lastSegmentIndex >= 0) {
    const uuid = threadNamespace.slice(lastSegmentIndex + THREAD_PATH_SEGMENT.length).trim();
    return uuid || null;
  }

  return null;
};

const extractThreadRelationFromTags = (
  tags: string[][],
): { rootEventId: string | null; parentEventId: string | null } => {
  let rootEventId: string | null = null;
  let parentEventId: string | null = null;

  tags.forEach((tag) => {
    if (tag[0] !== 'e') {
      return;
    }

    const referencedEventId = tag[1]?.trim();
    if (!referencedEventId) {
      return;
    }

    const marker = tag[3]?.trim();
    if (marker === 'root') {
      rootEventId = referencedEventId;
      return;
    }

    if (marker === 'reply') {
      parentEventId = referencedEventId;
      return;
    }

    if (!parentEventId) {
      parentEventId = referencedEventId;
    }
  });

  return { rootEventId, parentEventId };
};

interface ThreadDetails {
  tags: string[][];
  threadUuid: string;
  threadNamespace: string;
  threadRootEventId: string;
  threadParentEventId: string | null;
  isReply: boolean;
}

const resolveThreadDetails = (topicId: string, message: P2PMessage): ThreadDetails => {
  const tags = normalizeMessageTags(message.tags);
  const threadUuid =
    extractThreadUuidFromTags(tags, topicId) ?? deriveThreadUuidFromEventId(message.id);
  const threadNamespace =
    findTagValue(tags, 'thread') ?? `${topicId}${THREAD_PATH_SEGMENT}${threadUuid}`;
  const { rootEventId, parentEventId } = extractThreadRelationFromTags(tags);
  const isReply = parentEventId !== null;

  return {
    tags,
    threadUuid,
    threadNamespace,
    threadRootEventId: rootEventId ?? parentEventId ?? message.id,
    threadParentEventId: parentEventId,
    isReply,
  };
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
  const kind =
    typeof parsed.kind === 'number' && Number.isFinite(parsed.kind) ? parsed.kind : undefined;
  const tags = normalizeMessageTags(parsed.tags);

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
    kind,
    tags: tags.length > 0 ? tags : undefined,
  };
};

const isTopicPostMessage = (topicId: string, message: P2PMessage): boolean => {
  if (message.kind === NostrEventKind.TopicPost || message.kind === NostrEventKind.TextNote) {
    return true;
  }

  if (typeof message.kind === 'number') {
    return false;
  }

  const tags = normalizeMessageTags(message.tags);
  const taggedTopicId = findTagValue(tags, 't') ?? findTagValue(tags, 'topic');
  if (taggedTopicId === topicId) {
    return true;
  }

  return findTagValue(tags, 'thread_uuid') !== null || findTagValue(tags, 'thread') !== null;
};

const resolveAuthorNpub = async (author: string): Promise<string> => {
  if (!isHexFormat(author)) {
    return author;
  }
  return await pubkeyToNpub(author);
};

export function useP2PEventListener() {
  const queryClient = useQueryClient();
  const { addMessage, updatePeer, removePeer, refreshStatus } = useP2PStore();
  const { addPost } = usePostStore();
  const { updateTopicPostCount } = useTopicStore();
  const recentMessageIds = useRef<Set<string>>(new Set());
  const recentMessageOrder = useRef<string[]>([]);
  const authorProfileCache = useRef<Map<string, User>>(new Map());
  const authorProfileMissedAt = useRef<Map<string, number>>(new Map());
  const authorProfileInFlight = useRef<Map<string, Promise<User | null>>>(new Map());

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

  const resolveAuthor = useCallback(async (author: string): Promise<User> => {
    const cacheKey = author.trim().toLowerCase();

    const toFallbackAuthor = async (): Promise<User> => {
      const authorNpub = await resolveAuthorNpub(author);
      const fallbackName = shortenIdentifier(authorNpub || author);
      return applyKnownUserMetadata({
        id: author,
        pubkey: author,
        npub: authorNpub,
        name: fallbackName,
        displayName: fallbackName,
        about: '',
        picture: '',
        nip05: '',
        avatar: null,
        publicProfile: true,
        showOnlineStatus: false,
      });
    };

    const cached = authorProfileCache.current.get(cacheKey);
    if (cached) {
      return cached;
    }

    const missedAt = authorProfileMissedAt.current.get(cacheKey);
    if (missedAt && Date.now() - missedAt < AUTHOR_PROFILE_MISS_TTL_MS) {
      return await toFallbackAuthor();
    }

    const inFlight = authorProfileInFlight.current.get(cacheKey);
    if (inFlight) {
      const resolved = await inFlight;
      return resolved ?? (await toFallbackAuthor());
    }

    const loader = (async (): Promise<User | null> => {
      try {
        const profile = isHexFormat(author)
          ? await TauriApi.getUserProfileByPubkey(author)
          : author.startsWith('npub1')
            ? await TauriApi.getUserProfile(author)
            : null;
        if (!profile) {
          authorProfileMissedAt.current.set(cacheKey, Date.now());
          return null;
        }
        const mapped = mapUserProfileToUser(profile);
        const enriched = applyKnownUserMetadata(mapped);
        authorProfileCache.current.set(cacheKey, enriched);
        authorProfileMissedAt.current.delete(cacheKey);
        return enriched;
      } catch (error) {
        errorHandler.log('Failed to resolve author profile for P2P message', error, {
          context: 'useP2PEventListener.resolveAuthor',
          showToast: false,
          metadata: { author },
        });
        authorProfileMissedAt.current.set(cacheKey, Date.now());
        return null;
      }
    })();

    authorProfileInFlight.current.set(cacheKey, loader);

    try {
      const resolved = await loader;
      return resolved ?? (await toFallbackAuthor());
    } finally {
      authorProfileInFlight.current.delete(cacheKey);
    }
  }, []);

  const handleP2PMessageAsPost = useCallback(
    async (message: P2PMessage, topicId: string) => {
      try {
        const author = await resolveAuthor(message.author);
        const threadDetails = resolveThreadDetails(topicId, message);
        const createdAt =
          message.timestamp > 1_000_000_000_000
            ? Math.floor(message.timestamp / 1000)
            : Math.floor(message.timestamp);

        const post: Post = {
          id: message.id,
          content: message.content,
          author: author,
          topicId,
          threadNamespace: threadDetails.threadNamespace,
          threadUuid: threadDetails.threadUuid,
          threadRootEventId: threadDetails.threadRootEventId,
          threadParentEventId: threadDetails.threadParentEventId,
          created_at: createdAt,
          tags: threadDetails.tags.map((tag) => tag.join(':')),
          likes: 0,
          boosts: 0,
          replies: [],
          isSynced: true,
        };

        addPost(post);
        queryClient.setQueryData<Post[]>(['timeline'], (prev) => upsertPostIntoList(prev, post));
        queryClient.setQueryData<Post[]>(['posts', 'all'], (prev) =>
          upsertPostIntoList(prev, post),
        );
        queryClient.setQueryData<Post[]>(['posts', topicId], (prev) =>
          upsertPostIntoList(prev, post),
        );
        const threadUuid = post.threadUuid ?? post.id;
        queryClient.setQueryData<TopicTimelineEntry[]>(['topicTimeline', topicId], (prev) =>
          upsertTimelineEntry(prev, post, threadUuid, threadDetails.isReply),
        );
        queryClient.setQueryData<TopicTimelineEntry[]>(['topicThreads', topicId], (prev) =>
          upsertTimelineEntry(prev, post, threadUuid, threadDetails.isReply),
        );
        queryClient.setQueryData<Post[]>(['threadPosts', topicId, threadUuid], (prev) =>
          upsertPostIntoList(prev, post),
        );
        const timelineUpdateMode = useUIStore.getState().timelineUpdateMode;
        if (timelineUpdateMode === 'realtime') {
          const realtimeTags = [...threadDetails.tags];
          if (!realtimeTags.some((tag) => tag[0] === 't' && tag[1] === topicId)) {
            realtimeTags.push(['t', topicId]);
          }
          if (!realtimeTags.some((tag) => tag[0] === 'thread_uuid')) {
            realtimeTags.push(['thread_uuid', threadUuid]);
          }
          if (!realtimeTags.some((tag) => tag[0] === 'thread')) {
            realtimeTags.push(['thread', threadDetails.threadNamespace]);
          }
          if (!realtimeTags.some((tag) => tag[0] === 'source')) {
            realtimeTags.push(['source', 'p2p']);
          }

          dispatchTimelineRealtimeDelta({
            source: 'nostr',
            payload: {
              id: message.id,
              author: message.author,
              content: message.content,
              created_at: message.timestamp,
              kind: NostrEventKind.TopicPost,
              tags: realtimeTags,
            },
          });
        }
        updateTopicPostCount(topicId, 1);
        const invalidateInBackground = (queryKey: readonly unknown[]) =>
          queryClient.invalidateQueries({ queryKey, refetchType: 'active' });
        void invalidateInBackground(['posts', topicId]);
        void invalidateInBackground(['topicTimeline', topicId]);
        void invalidateInBackground(['topicThreads', topicId]);
        void invalidateInBackground(['threadPosts', topicId]);
      } catch (error) {
        errorHandler.log('Failed to process P2P message as post', error, {
          context: 'useP2PEventListener.handleP2PMessageAsPost',
          showToast: false,
        });
      }
    },
    [addPost, queryClient, resolveAuthor, updateTopicPostCount],
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

      if (!isTopicPostMessage(topic_id, p2pMessage)) {
        return;
      }

      addMessage(p2pMessage);

      const messageTimestampSeconds =
        p2pMessage.timestamp > 1_000_000_000_000
          ? Math.floor(p2pMessage.timestamp / 1000)
          : Math.floor(p2pMessage.timestamp);

      useTopicStore.getState().handleIncomingTopicMessage(topic_id, messageTimestampSeconds);
      void handleP2PMessageAsPost(p2pMessage, topic_id);
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

    void registerListener<P2PRawMessageEvent>(
      'p2p://message/raw',
      ({ topic_id, payload, timestamp }) => {
        const parsed = parseRawEventMessage(topic_id, payload, timestamp);
        if (!parsed) {
          errorHandler.log('Drop unparsable P2P raw payload', undefined, {
            context: 'useP2PEventListener.p2p://message/raw',
            showToast: false,
            metadata: {
              topic_id,
              payloadType: typeof payload,
            },
          });
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
