import type {
  ChannelRef,
  PostView,
  TimelineCursor,
  TimelineScope,
} from '@/lib/api';

import {
  type DraftMediaItem,
  PUBLIC_CHANNEL_REF,
  PUBLIC_TIMELINE_SCOPE,
  STARTER_TOPICS,
  buildStarterTopicRecord,
  timelineScopeStorageKey,
} from '@/shell/slices/shared';

/// タイムライン・スレッド・投稿コンポーザ(WP-H6 PR3 のドメインスライス)。
export type TimelineSliceState = {
  trackedTopics: string[];
  activeTopic: string;
  topicInput: string;
  composer: string;
  draftMediaItems: DraftMediaItem[];
  attachmentInputKey: number;
  timelinesByKey: Record<string, PostView[]>;
  timelineNextCursorByKey: Record<string, TimelineCursor | null>;
  timelineLoadingMoreByKey: Record<string, boolean>;
  pendingTimelineSnapshotsByKey: Record<string, PostView[]>;
  pendingTimelineCountsByKey: Record<string, number>;
  pendingTimelineNextCursorByKey: Record<string, TimelineCursor | null>;
  publicTimelinesByTopic: Record<string, PostView[]>;
  publicTimelineNextCursorByTopic: Record<string, TimelineCursor | null>;
  publicTimelineLoadingMoreByTopic: Record<string, boolean>;
  timelineScopeByTopic: Record<string, TimelineScope>;
  composeChannelByTopic: Record<string, ChannelRef>;
  thread: PostView[];
  threadsById: Record<string, PostView[]>;
  threadNextCursorById: Record<string, TimelineCursor | null>;
  threadLoadingMoreById: Record<string, boolean>;
  selectedThread: string | null;
  focusedObjectId: string | null;
  replyTarget: PostView | null;
  repostTarget: PostView | null;
  composerError: string | null;
};

export function createInitialTimelineSlice(): TimelineSliceState {
  return {
    trackedTopics: [...STARTER_TOPICS],
    activeTopic: STARTER_TOPICS[0],
    topicInput: '',
    composer: '',
    draftMediaItems: [],
    attachmentInputKey: 0,
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
    publicTimelinesByTopic: buildStarterTopicRecord(() => [] as PostView[]),
    publicTimelineNextCursorByTopic: buildStarterTopicRecord(() => null as TimelineCursor | null),
    publicTimelineLoadingMoreByTopic: buildStarterTopicRecord(() => false),
    timelineScopeByTopic: buildStarterTopicRecord(() => ({ ...PUBLIC_TIMELINE_SCOPE })),
    composeChannelByTopic: buildStarterTopicRecord(() => ({ ...PUBLIC_CHANNEL_REF })),
    thread: [],
    threadsById: {},
    threadNextCursorById: {},
    threadLoadingMoreById: {},
    selectedThread: null,
    focusedObjectId: null,
    replyTarget: null,
    repostTarget: null,
    composerError: null,
  };
}
