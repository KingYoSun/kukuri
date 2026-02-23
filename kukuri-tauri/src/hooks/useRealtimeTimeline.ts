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
import type { NostrEventPayload } from '@/types/nostr';
import { collectTimelineStorePosts, type TopicTimelineEntry } from './usePosts';

const REALTIME_BATCH_INTERVAL_MS = 750;
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
  threadUuid: string,
  entries: TopicTimelineEntry[],
): Post => {
  const { rootEventId, parentEventId } = extractThreadRelation(payload.tags);
  const createdAt = toUnixSeconds(payload.created_at);

  return {
    id: payload.id,
    content: payload.content,
    author: resolveFallbackAuthor(entries, payload.author, 'Realtime user'),
    topicId,
    threadNamespace: `${topicId}${THREAD_PATH_SEGMENT}${threadUuid}`,
    threadUuid,
    threadRootEventId: rootEventId ?? payload.id,
    threadParentEventId: parentEventId,
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
  if (payload.kind !== 30078) {
    return { status: 'ignored' };
  }

  const taggedTopicId = extractTopicIdFromTags(payload.tags);
  if (!taggedTopicId || taggedTopicId !== topicId) {
    return { status: 'ignored' };
  }

  const threadUuid = extractThreadUuid(payload.tags, topicId);
  if (!threadUuid) {
    return { status: 'requires_refetch' };
  }

  const { parentEventId } = extractThreadRelation(payload.tags);
  const isReply = parentEventId !== null;
  const createdAt = toUnixSeconds(payload.created_at);
  const realtimePost = toRealtimePost(payload, topicId, threadUuid, entries);

  const existingIndex = entries.findIndex((entry) => entry.threadUuid === threadUuid);
  if (existingIndex < 0) {
    if (isReply) {
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

  const updatedEntry: TopicTimelineEntry = isReply
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
  const queuedDeltasRef = useRef<TimelineRealtimeDelta[]>([]);
  const flushTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const seenRealtimePostIdsRef = useRef<Set<string>>(new Set());
  const fallbackTriggeredRef = useRef(false);

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

    if (currentEntries.length === 0) {
      void queryClient.invalidateQueries({ queryKey });
      return;
    }

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
    if (mode === 'realtime') {
      fallbackTriggeredRef.current = false;
      queuedDeltasRef.current = [];
      seenRealtimePostIdsRef.current.clear();
      return;
    }

    queuedDeltasRef.current = [];
    seenRealtimePostIdsRef.current.clear();
    if (flushTimerRef.current) {
      clearTimeout(flushTimerRef.current);
      flushTimerRef.current = null;
    }
  }, [mode, topicId]);

  useEffect(() => {
    if (mode !== 'realtime') {
      return;
    }

    const shouldFallback =
      connectionStatus === 'disconnected' ||
      connectionStatus === 'error' ||
      (typeof navigator !== 'undefined' && navigator.onLine === false);

    if (!shouldFallback || fallbackTriggeredRef.current) {
      return;
    }

    fallbackTriggeredRef.current = true;
    onFallbackToStandard();
  }, [connectionStatus, mode, onFallbackToStandard]);

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
