import type {
  AuthorDetailView,
  PostCardView,
  PostMediaView,
  ThreadPanelState,
  TopicDiagnosticSummary,
} from '@/components/core/types';
import type { PrimarySection } from '@/components/shell/types';

export const STORY_ACTIVE_TOPIC = 'kukuri:topic:demo';

export const STORY_PRIMARY_ITEMS: Array<{
  id: PrimarySection;
  label: string;
  description: string;
}> = [
  { id: 'timeline', label: 'Timeline', description: 'Feed and scope controls' },
  { id: 'channels', label: 'Channels', description: 'Private channel entry and composer' },
  { id: 'live', label: 'Live', description: 'Live sessions and status' },
  { id: 'game', label: 'Game', description: 'Scoreboards and room editing' },
  { id: 'profile', label: 'Profile', description: 'Edit author identity' },
];

export const STORY_TOPIC_ITEMS: TopicDiagnosticSummary[] = [
  {
    topic: STORY_ACTIVE_TOPIC,
    active: true,
    removable: false,
    connectionLabel: 'joined',
    peerCount: 2,
    lastReceivedLabel: '12:45:11',
  },
  {
    topic: 'kukuri:topic:relay',
    active: false,
    removable: true,
    connectionLabel: 'relay-assisted',
    peerCount: 1,
    lastReceivedLabel: 'no events',
  },
];

export const STORY_IMAGE_PREVIEW =
  'data:image/svg+xml;utf8,<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 640 360"><rect width="640" height="360" fill="%2300b3a4"/><path d="M0 300l140-130 100 80 110-120 150 170H0z" fill="%230f2231"/></svg>';

export const STORY_VIDEO_POSTER_PREVIEW =
  'data:image/svg+xml;utf8,<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 640 360"><rect width="640" height="360" fill="%23101923"/><rect x="110" y="70" width="420" height="220" rx="24" fill="%23f59d62"/><polygon points="285,145 285,215 355,180" fill="%23101923"/></svg>';

export const STORY_VIDEO_PLAYBACK_SRC =
  'data:video/mp4;base64,AAAAIGZ0eXBpc29tAAACAGlzb21pc28yYXZjMW1wNDEAAABsbXZoZAAAAAAAAAAAAAAAAAAAA+gAAAPoAAEAAAEAAAAAAAAAAAAAAAABAAAAAAAAAAAAAAAAAAAAAQAAAAAAAAAAAAAAAAAAQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAIAAAIVdHJhawAAAFx0a2hkAAAAAwAAAAAAAAAAAAAAAQAAAAAAAAAAAAAAAAEAAAAAAAAAAAAAAAAAAAAAAAEAAAAAAAAAAAAAAAAAAAABAAAAAAAAAAAAAAAAAABAAAAAAQAAAEAAAAAAAAkZWR0cwAAABxlbHN0AAAAAAAAAAEAAAPoAAAAAAABAAAAAAEAAAAqbWRpYQAAACBtZGhkAAAAAAAAAAAAAAAAAAAyAAAAMgBVxAAAAAAALWhkbHIAAAAAAAAAAHZpZGUAAAAAAAAAAAAAAABWaWRlb0hhbmRsZXIAAAAClW1pbmYAAAAUdm1oZAAAAAEAAAAAAAAAAAAAACRkaW5mAAAAHGRyZWYAAAAAAAAAAQAAAAx1cmwgAAAAAQAAAl1zdGJsAAAArXN0c2QAAAAAAAAAAQAAAJ1hdmMxAAAAAAAAAAEAAAAAAAAAAAAAAAAAAAAAAQABAAABAAABAAAAAAAAAAAAAAAAAQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABj//wAAADNhdmNDAWQAHv/hABdnZAAerNlChAAAAMAEAAAAwHihUkA=';

const timelineRootPost = {
  object_id: 'timeline-post-1',
  envelope_id: 'env-timeline-post-1',
  author_pubkey: 'b'.repeat(64),
  author_name: 'bob',
  author_display_name: null,
  following: true,
  followed_by: true,
  mutual: true,
  friend_of_friend: false,
  object_kind: 'post' as const,
  content: 'Core workspace now owns topic switching, composer, and thread context.',
  content_status: 'Available' as const,
  attachments: [],
  created_at: 1,
  reply_to: null,
  root_id: 'timeline-post-1',
  channel_id: null,
  audience_label: 'Public',
};

export const STORY_TIMELINE_POSTS: PostCardView[] = [
  {
    post: timelineRootPost,
    context: 'timeline',
    authorLabel: 'bob',
    authorPicture: null,
    relationshipLabel: 'mutual',
    audienceChipLabel: 'Public',
    threadTargetId: 'timeline-post-1',
    media: {
      objectId: 'timeline-post-1',
      kind: null,
      statusLabel: null,
      extraAttachmentCount: 0,
      state: 'ready',
      metaMime: null,
      metaBytesLabel: null,
      imagePreviewSrc: null,
      videoPosterPreviewSrc: null,
      videoPlaybackSrc: null,
      videoUnsupportedOnClient: false,
    },
  },
  {
    post: {
      object_id: 'timeline-post-2',
      envelope_id: 'env-timeline-post-2',
      author_pubkey: 'c'.repeat(64),
      author_name: 'carol',
      author_display_name: 'Carol',
      following: false,
      followed_by: false,
      mutual: false,
      friend_of_friend: true,
      object_kind: 'post',
      content: 'Image preview stays visible before the blob finishes syncing.',
      content_status: 'Available',
      attachments: [],
      created_at: 2,
      reply_to: null,
      root_id: 'timeline-post-2',
      channel_id: null,
      audience_label: 'Public',
    },
    context: 'timeline',
    authorLabel: 'Carol',
    authorPicture: null,
    relationshipLabel: 'friend of friend',
    audienceChipLabel: 'Public',
    threadTargetId: 'timeline-post-2',
    media: {
      objectId: 'timeline-post-2',
      kind: 'image',
      statusLabel: 'image ready',
      extraAttachmentCount: 0,
      state: 'ready',
      metaMime: 'image/png',
      metaBytesLabel: '144 KB',
      imagePreviewSrc: STORY_IMAGE_PREVIEW,
      videoPosterPreviewSrc: null,
      videoPlaybackSrc: null,
      videoUnsupportedOnClient: false,
    },
  },
];

export const STORY_THREAD_POSTS: PostCardView[] = [
  {
    ...STORY_TIMELINE_POSTS[0],
    context: 'thread',
  },
  {
    post: {
      object_id: 'thread-reply-1',
      envelope_id: 'env-thread-reply-1',
      author_pubkey: 'd'.repeat(64),
      author_name: 'dan',
      author_display_name: null,
      following: false,
      followed_by: true,
      mutual: false,
      friend_of_friend: false,
      object_kind: 'comment',
      content: 'Reply mode keeps the audience label and lets the user publish in place.',
      content_status: 'Available',
      attachments: [],
      created_at: 3,
      reply_to: 'timeline-post-1',
      root_id: 'timeline-post-1',
      channel_id: null,
      audience_label: 'Public',
    },
    context: 'thread',
    authorLabel: 'dan',
    authorPicture: null,
    relationshipLabel: 'follows you',
    audienceChipLabel: 'Public',
    threadTargetId: 'timeline-post-1',
    media: {
      objectId: 'thread-reply-1',
      kind: null,
      statusLabel: null,
      extraAttachmentCount: 0,
      state: 'ready',
      metaMime: null,
      metaBytesLabel: null,
      imagePreviewSrc: null,
      videoPosterPreviewSrc: null,
      videoPlaybackSrc: null,
      videoUnsupportedOnClient: false,
    },
  },
];

export const STORY_THREAD_PANEL_STATE: ThreadPanelState = {
  selectedThreadId: 'timeline-post-1',
  summary: '2 posts in thread',
  emptyCopy: 'Select a post to inspect the thread.',
};

export const STORY_AUTHOR_DETAIL_VIEW: AuthorDetailView = {
  author: {
    author_pubkey: 'b'.repeat(64),
    name: 'bob',
    display_name: null,
    about: 'Maintains the topic-first shell and community-node connectivity reviews.',
    picture: null,
    updated_at: 1,
    following: true,
    followed_by: true,
    mutual: true,
    friend_of_friend: false,
    friend_of_friend_via_pubkeys: [],
  },
  displayLabel: 'bob',
  summary: {
    label: 'mutual',
    following: true,
    followedBy: true,
    mutual: true,
    friendOfFriend: false,
    viaPubkeys: [],
    isSelf: false,
    canFollow: true,
    followActionLabel: 'Unfollow',
  },
  authorError: null,
};

export const STORY_EMPTY_AUTHOR_DETAIL_VIEW: AuthorDetailView = {
  author: null,
  displayLabel: '',
  summary: null,
  authorError: null,
};

export const STORY_IMAGE_MEDIA: PostMediaView = {
  objectId: 'timeline-post-2',
  kind: 'image',
  statusLabel: 'image ready',
  extraAttachmentCount: 0,
  state: 'ready',
  metaMime: 'image/png',
  metaBytesLabel: '144 KB',
  imagePreviewSrc: STORY_IMAGE_PREVIEW,
  videoPosterPreviewSrc: null,
  videoPlaybackSrc: null,
  videoUnsupportedOnClient: false,
};

export const STORY_VIDEO_POSTER_MEDIA: PostMediaView = {
  objectId: 'video-post',
  kind: 'video',
  statusLabel: 'poster ready',
  extraAttachmentCount: 1,
  state: 'ready',
  metaMime: 'video/mp4',
  metaBytesLabel: '8.0 KB',
  imagePreviewSrc: null,
  videoPosterPreviewSrc: STORY_VIDEO_POSTER_PREVIEW,
  videoPlaybackSrc: null,
  videoUnsupportedOnClient: false,
};

export const STORY_VIDEO_PLAYABLE_MEDIA: PostMediaView = {
  objectId: 'video-post',
  kind: 'video',
  statusLabel: 'playable video',
  extraAttachmentCount: 0,
  state: 'ready',
  metaMime: 'video/mp4',
  metaBytesLabel: '8.0 KB',
  imagePreviewSrc: null,
  videoPosterPreviewSrc: STORY_VIDEO_POSTER_PREVIEW,
  videoPlaybackSrc: STORY_VIDEO_PLAYBACK_SRC,
  videoUnsupportedOnClient: false,
};
