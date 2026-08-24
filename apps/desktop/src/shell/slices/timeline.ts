import type {
  ChannelRef,
  PostView,
  TimelineCursor,
  TimelineScope,
} from '@/lib/api';

import {
  PUBLIC_CHANNEL_REF,
  PUBLIC_TIMELINE_SCOPE,
  STARTER_TOPICS,
  buildStarterTopicRecord,
  timelineScopeStorageKey,
} from '@/shell/slices/shared';

/// タイムライン・スレッド・投稿コンポーザ(WP-H6 PR3 のドメインスライス)。
export type TimelineSliceState = {
  trackedTopics: string[];
  topicInput: string;
  timelinesByKey: Record<string, PostView[]>;
  timelineNextCursorByKey: Record<string, TimelineCursor | null>;
  timelineLoadingMoreByKey: Record<string, boolean>;
  pendingTimelineSnapshotsByKey: Record<string, PostView[]>;
  pendingTimelineCountsByKey: Record<string, number>;
  pendingTimelineNextCursorByKey: Record<string, TimelineCursor | null>;
  timelineScopeByTopic: Record<string, TimelineScope>;
  composeChannelByTopic: Record<string, ChannelRef>;
  threadsById: Record<string, PostView[]>;
  threadNextCursorById: Record<string, TimelineCursor | null>;
  threadLoadingMoreById: Record<string, boolean>;
  selectedThread: string | null;
  focusedObjectId: string | null;
};

export function createInitialTimelineSlice(): TimelineSliceState {
  return {
    trackedTopics: [...STARTER_TOPICS],
    topicInput: '',
    timelinesByKey: Object.fromEntries(
      STARTER_TOPICS.map((topic) => [timelineScopeStorageKey(topic, PUBLIC_TIMELINE_SCOPE), []])
    ),
    timelineNextCursorByKey: Object.fromEntries(
      STARTER_TOPICS.map((topic) => [timelineScopeStorageKey(topic, PUBLIC_TIMELINE_SCOPE), null])
    ),
    timelineLoadingMoreByKey: Object.fromEntries(
      STARTER_TOPICS.map((topic) => [timelineScopeStorageKey(topic, PUBLIC_TIMELINE_SCOPE), false])
    ),
    pendingTimelineSnapshotsByKey: {},
    pendingTimelineCountsByKey: {},
    pendingTimelineNextCursorByKey: {},
    timelineScopeByTopic: buildStarterTopicRecord(() => ({ ...PUBLIC_TIMELINE_SCOPE })),
    composeChannelByTopic: buildStarterTopicRecord(() => ({ ...PUBLIC_CHANNEL_REF })),
    threadsById: {},
    threadNextCursorById: {},
    threadLoadingMoreById: {},
    selectedThread: null,
    focusedObjectId: null,
  };
}
