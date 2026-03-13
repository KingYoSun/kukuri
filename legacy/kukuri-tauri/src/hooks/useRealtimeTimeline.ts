import { useCallback, useEffect, useRef } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import type { Post, User } from '@/stores/types';
import { applyKnownUserMetadata } from '@/lib/profile/userMetadata';
import { errorHandler } from '@/lib/errorHandler';
import { usePostStore } from '@/stores/postStore';
import { useP2PStore } from '@/stores/p2pStore';
import {
  TIMELINE_REALTIME_DELTA_EVENT,
  type TimelineRealtimeDelta,
} from '@/lib/realtime/timelineRealtimeEvents';
import type { TimelineUpdateMode } from '@/stores/uiStore';
import { NostrEventKind, type NostrEventPayload } from '@/types/nostr';
import { collectTimelineStorePosts, type TopicTimelineEntry } from './usePosts';

const REALTIME_BATCH_INTERVAL_MS = 750;
const REALTIME_DISCONNECTED_FALLBACK_GRACE_MS = 15_000;
const THREAD_PATH_SEGMENT = '/threads/';

interface UseRealtimeTimelineOptions {
  topicId: string;
  mode: TimelineUpdateMode;
  onFallbackToStandard: () => void;
}

type DeltaApplyResult =
  | { status: 'ignored' }
  | { status: 'requires_refetch' }
  | { status: 'updated'; entries: TopicTimelineEntry[] };

const toUnixSeconds = (timestamp: number): number =>
  timestamp > 1_000_000_000_000 ? Math.floor(timestamp / 1000) : Math.floor(timestamp);

const sortTimelineEntries = (entries: TopicTimelineEntry[]): TopicTimelineEntry[] =>
  [...entries].sort((a, b) => b.lastActivityAt - a.lastActivityAt);

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

  bytes[6] = (bytes[6] & 0x0f) | 0x50;
  bytes[8] = (bytes[8] & 0x3f) | 0x80;

  const toHex = (value: number): string => value.toString(16).padStart(2, '0');
  const digest = Array.from(bytes, toHex).join('');
  return `${digest.slice(0, 8)}-${digest.slice(8, 12)}-${digest.slice(12, 16)}-${digest.slice(16, 20)}-${digest.slice(20, 32)}`;
};

const shortenAuthorLabel = (value: string): string => {
  const trimmed = value.trim();
  if (!trimmed) {
    return 'P2P user';
  }
  if (trimmed.length <= 16) {
    return trimmed;
  }
  return `${trimmed.slice(0, 8)}...${trimmed.slice(-4)}`;
};

const findTagValue = (tags: string[][], key: string): string | null => {
  const value = tags.find((tag) => tag[0] === key)?.[1];
  const trimmed = value?.trim();
  return trimmed ? trimmed : null;
};

const extractTopicIdFromTags = (tags: string[][]): string | null => findTagValue(tags, 't');

const extractThreadUuid = (tags: string[][], topicId: string): string | null => {
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

const extractThreadRelation = (
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

const trimNonEmpty = (value: string | null | undefined): string | null => {
  const trimmed = value?.trim();
  return trimmed ? trimmed : null;
};

const getTopicPostsFromStore = (topicId: string): Post[] => {
  const postStore = usePostStore.getState();
  const topicPostIds = postStore.postsByTopic.get(topicId) ?? [];
  return topicPostIds
    .map((postId) => postStore.posts.get(postId))
    .filter((post): post is Post => Boolean(post));
};

const matchesEventReference = (post: Post, eventId: string): boolean => {
  const target = eventId.trim();
  if (!target) {
    return false;
  }
  return post.id === target || post.eventId?.trim() === target;
};

const matchesRootReference = (post: Post, eventId: string): boolean =>
  matchesEventReference(post, eventId) || post.threadRootEventId?.trim() === eventId.trim();

const resolveStoredThreadContext = (
  topicId: string,
  rootEventId: string | null,
  parentEventId: string | null,
): {
  threadUuid: string;
  threadNamespace: string;
  threadRootEventId: string;
  threadParentEventId: string | null;
} | null => {
  const topicPosts = getTopicPostsFromStore(topicId);
  const parentTarget = trimNonEmpty(parentEventId);
  const rootTarget = trimNonEmpty(rootEventId);
  const matchedParent = parentTarget
    ? (topicPosts.find((post) => matchesEventReference(post, parentTarget)) ?? null)
    : null;
  const matchedRoot = rootTarget
    ? (topicPosts.find((post) => matchesRootReference(post, rootTarget)) ?? null)
    : null;
  const source = matchedParent ?? matchedRoot;
  if (!source) {
    return null;
  }

  const threadRootEventId =
    trimNonEmpty(source.threadRootEventId) ?? trimNonEmpty(source.eventId) ?? source.id;
  const threadUuid =
    trimNonEmpty(source.threadUuid) ?? deriveThreadUuidFromEventId(threadRootEventId);
  const threadNamespace =
    trimNonEmpty(source.threadNamespace) ?? `${topicId}${THREAD_PATH_SEGMENT}${threadUuid}`;

  return {
    threadUuid,
    threadNamespace,
    threadRootEventId,
    threadParentEventId: parentTarget,
  };
};

interface RealtimeThreadDetails {
  threadUuid: string;
  threadNamespace: string;
  threadRootEventId: string;
  threadParentEventId: string | null;
  isReply: boolean;
}

const resolveRealtimeThreadDetails = (
  payload: NostrEventPayload,
  topicId: string,
): RealtimeThreadDetails => {
  const { rootEventId, parentEventId } = extractThreadRelation(payload.tags);
  const storedThreadContext = resolveStoredThreadContext(topicId, rootEventId, parentEventId);
  const threadRootEventId =
    storedThreadContext?.threadRootEventId ?? rootEventId ?? parentEventId ?? payload.id;
  const threadUuid =
    extractThreadUuid(payload.tags, topicId) ??
    storedThreadContext?.threadUuid ??
    deriveThreadUuidFromEventId(threadRootEventId);
  const threadNamespace =
    findTagValue(payload.tags, 'thread') ??
    storedThreadContext?.threadNamespace ??
    `${topicId}${THREAD_PATH_SEGMENT}${threadUuid}`;
  const threadParentEventId = parentEventId ?? storedThreadContext?.threadParentEventId ?? null;

  return {
    threadUuid,
    threadNamespace,
    threadRootEventId,
    threadParentEventId,
    isReply: threadParentEventId !== null,
  };
};

const resolveFallbackAuthor = (
  entries: TopicTimelineEntry[],
  authorPubkey: string,
  fallbackName: string,
): User => {
  const existingAuthor =
    entries
      .flatMap(
        (entry) => [entry.parentPost.author, entry.firstReply?.author].filter(Boolean) as User[],
      )
      .find((author) => author.pubkey === authorPubkey) ?? null;

  if (existingAuthor) {
    return existingAuthor;
  }

  return applyKnownUserMetadata({
    id: authorPubkey,
    pubkey: authorPubkey,
    npub: authorPubkey,
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

const toRealtimePost = (
  payload: NostrEventPayload,
  topicId: string,
  threadDetails: RealtimeThreadDetails,
  entries: TopicTimelineEntry[],
): Post => {
  const createdAt = toUnixSeconds(payload.created_at);

  return {
    id: payload.id,
    eventId: payload.id,
    content: payload.content,
    author: resolveFallbackAuthor(entries, payload.author, shortenAuthorLabel(payload.author)),
    topicId,
    threadNamespace: threadDetails.threadNamespace,
    threadUuid: threadDetails.threadUuid,
    threadRootEventId: threadDetails.threadRootEventId,
    threadParentEventId: threadDetails.threadParentEventId,
    created_at: createdAt,
    tags: payload.tags.map((tag) => tag.join(':')),
    likes: 0,
    boosts: 0,
    replies: [],
    replyCount: 0,
    isSynced: true,
  };
};

const applyNostrDeltaToTimeline = (
  entries: TopicTimelineEntry[],
  payload: NostrEventPayload,
  topicId: string,
): DeltaApplyResult => {
  if (payload.kind !== NostrEventKind.TopicPost && payload.kind !== NostrEventKind.TextNote) {
    return { status: 'ignored' };
  }

  const taggedTopicId = extractTopicIdFromTags(payload.tags);
  if (!taggedTopicId || taggedTopicId !== topicId) {
    return { status: 'ignored' };
  }

  const threadDetails = resolveRealtimeThreadDetails(payload, topicId);
  const createdAt = toUnixSeconds(payload.created_at);
  const realtimePost = toRealtimePost(payload, topicId, threadDetails, entries);
  const threadUuid = threadDetails.threadUuid;

  const existingIndex = entries.findIndex((entry) => entry.threadUuid === threadUuid);
  if (existingIndex < 0) {
    if (threadDetails.isReply) {
      return { status: 'requires_refetch' };
    }

    const newEntry: TopicTimelineEntry = {
      threadUuid,
      parentPost: realtimePost,
      firstReply: null,
      replyCount: 0,
      lastActivityAt: createdAt,
    };
    return { status: 'updated', entries: sortTimelineEntries([newEntry, ...entries]) };
  }

  const currentEntry = entries[existingIndex];
  if (
    currentEntry.parentPost.id === realtimePost.id ||
    currentEntry.firstReply?.id === realtimePost.id
  ) {
    return { status: 'ignored' };
  }

  const updatedEntry: TopicTimelineEntry = threadDetails.isReply
    ? {
        ...currentEntry,
        firstReply: currentEntry.firstReply ?? realtimePost,
        replyCount: currentEntry.replyCount + 1,
        lastActivityAt: Math.max(currentEntry.lastActivityAt, createdAt),
      }
    : {
        ...currentEntry,
        parentPost: realtimePost,
        lastActivityAt: Math.max(currentEntry.lastActivityAt, createdAt),
      };

  const nextEntries = [...entries];
  nextEntries[existingIndex] = updatedEntry;
  return { status: 'updated', entries: sortTimelineEntries(nextEntries) };
};

const hasPostIdInEntries = (entries: TopicTimelineEntry[], postId: string): boolean =>
  entries.some((entry) => entry.parentPost.id === postId || entry.firstReply?.id === postId);

export function useRealtimeTimeline({
  topicId,
  mode,
  onFallbackToStandard,
}: UseRealtimeTimelineOptions): void {
  const queryClient = useQueryClient();
  const setPosts = usePostStore((state) => state.setPosts);
  const connectionStatus = useP2PStore((state) => state.connectionStatus);
  const activeTopicStats = useP2PStore((state) =>
    topicId ? (state.activeTopics.get(topicId) ?? null) : null,
  );
  const queuedDeltasRef = useRef<TimelineRealtimeDelta[]>([]);
  const flushTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const seenRealtimePostIdsRef = useRef<Set<string>>(new Set());
  const fallbackTriggeredRef = useRef(false);
  const realtimeActivatedAtRef = useRef<number | null>(null);

  const flushRealtimeQueue = useCallback(() => {
    flushTimerRef.current = null;

    if (mode !== 'realtime' || !topicId) {
      queuedDeltasRef.current = [];
      return;
    }

    const queuedDeltas = queuedDeltasRef.current.splice(0, queuedDeltasRef.current.length);
    if (queuedDeltas.length === 0) {
      return;
    }

    const queryKey = ['topicTimeline', topicId] as const;
    const currentEntries = queryClient.getQueryData<TopicTimelineEntry[]>(queryKey) ?? [];

    let nextEntries = currentEntries;
    let hasDiff = false;
    let needsRefetch = false;

    queuedDeltas.forEach((delta) => {
      if (delta.source === 'nostr') {
        if (seenRealtimePostIdsRef.current.has(delta.payload.id)) {
          return;
        }

        const result = applyNostrDeltaToTimeline(nextEntries, delta.payload, topicId);
        if (result.status === 'ignored') {
          return;
        }

        if (result.status === 'requires_refetch') {
          needsRefetch = true;
          seenRealtimePostIdsRef.current.add(delta.payload.id);
          return;
        }

        nextEntries = result.entries;
        hasDiff = true;
        seenRealtimePostIdsRef.current.add(delta.payload.id);
        return;
      }

      if (delta.topicId !== topicId) {
        return;
      }

      if (
        seenRealtimePostIdsRef.current.has(delta.message.id) ||
        hasPostIdInEntries(nextEntries, delta.message.id)
      ) {
        seenRealtimePostIdsRef.current.add(delta.message.id);
        return;
      }

      needsRefetch = true;
      seenRealtimePostIdsRef.current.add(delta.message.id);
    });

    if (hasDiff) {
      queryClient.setQueryData(queryKey, nextEntries);
      setPosts(collectTimelineStorePosts(nextEntries));
    }

    if (needsRefetch) {
      void queryClient.invalidateQueries({ queryKey });
    }
  }, [mode, queryClient, setPosts, topicId]);

  const scheduleFlush = useCallback(() => {
    if (flushTimerRef.current) {
      return;
    }

    flushTimerRef.current = setTimeout(() => {
      try {
        flushRealtimeQueue();
      } catch (error) {
        errorHandler.log('Failed to flush realtime timeline queue', error, {
          context: 'useRealtimeTimeline.flushRealtimeQueue',
        });
      }
    }, REALTIME_BATCH_INTERVAL_MS);
  }, [flushRealtimeQueue]);

  useEffect(() => {
    if (flushTimerRef.current) {
      clearTimeout(flushTimerRef.current);
      flushTimerRef.current = null;
    }

    queuedDeltasRef.current = [];
    seenRealtimePostIdsRef.current.clear();

    if (mode === 'realtime') {
      fallbackTriggeredRef.current = false;
      realtimeActivatedAtRef.current = Date.now();
      return;
    }
    realtimeActivatedAtRef.current = null;
  }, [mode, topicId]);

  useEffect(() => {
    if (mode !== 'realtime') {
      return;
    }

    const hasTopicActivity =
      (activeTopicStats?.peer_count ?? 0) > 0 || (activeTopicStats?.message_count ?? 0) > 0;
    const disconnectedGraceElapsed =
      realtimeActivatedAtRef.current !== null &&
      Date.now() - realtimeActivatedAtRef.current >= REALTIME_DISCONNECTED_FALLBACK_GRACE_MS;
    const shouldFallback =
      connectionStatus === 'error' ||
      (connectionStatus === 'disconnected' && disconnectedGraceElapsed && !hasTopicActivity) ||
      (typeof navigator !== 'undefined' && navigator.onLine === false);

    if (!shouldFallback || fallbackTriggeredRef.current) {
      return;
    }

    fallbackTriggeredRef.current = true;
    onFallbackToStandard();
  }, [activeTopicStats, connectionStatus, mode, onFallbackToStandard]);

  useEffect(() => {
    if (mode !== 'realtime') {
      return;
    }

    const handleOffline = () => {
      if (fallbackTriggeredRef.current) {
        return;
      }
      fallbackTriggeredRef.current = true;
      onFallbackToStandard();
    };

    window.addEventListener('offline', handleOffline);
    return () => {
      window.removeEventListener('offline', handleOffline);
    };
  }, [mode, onFallbackToStandard]);

  useEffect(() => {
    const handleRealtimeDelta = (event: Event) => {
      if (mode !== 'realtime' || !topicId) {
        return;
      }

      const customEvent = event as CustomEvent<TimelineRealtimeDelta>;
      const delta = customEvent.detail;
      if (!delta) {
        return;
      }

      if (delta.source === 'p2p' && delta.topicId !== topicId) {
        return;
      }

      if (delta.source === 'nostr') {
        const taggedTopicId = extractTopicIdFromTags(delta.payload.tags);
        if (!taggedTopicId || taggedTopicId !== topicId) {
          return;
        }
      }

      queuedDeltasRef.current.push(delta);
      scheduleFlush();
    };

    window.addEventListener(TIMELINE_REALTIME_DELTA_EVENT, handleRealtimeDelta as EventListener);
    return () => {
      window.removeEventListener(
        TIMELINE_REALTIME_DELTA_EVENT,
        handleRealtimeDelta as EventListener,
      );
    };
  }, [mode, scheduleFlush, topicId]);
}
