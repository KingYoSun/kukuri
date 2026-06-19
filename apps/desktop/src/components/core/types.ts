import type * as React from 'react';

import type { AuthorSocialView, ChannelAudienceKind, ContentProvenance, PostView } from '@/lib/api';

export type TopicChannelSummary = {
  channelId: string;
  label: string;
  audienceKind: ChannelAudienceKind;
  active: boolean;
  // Whether this channel is currently connected to the gossip network.
  // Absent is treated as connected.
  gossipJoined?: boolean;
};

export type TopicDiagnosticSummary = {
  topic: string;
  active: boolean;
  publicActive?: boolean;
  removable: boolean;
  connectionLabel: string;
  peerCount: number;
  lastReceivedLabel: string;
  // Raw last-received timestamp used for "updated" sorting in the nav list.
  // Null/absent means nothing has been received yet.
  lastReceivedAt?: number | null;
  // Whether this topic is currently connected to the gossip network.
  // Absent is treated as connected.
  gossipJoined?: boolean;
  channels?: TopicChannelSummary[];
};

export type ComposerDraftAttachmentView = {
  key: string;
  label: string;
  mime: string;
  byteSizeLabel: string;
};

export type ComposerDraftMediaView = {
  id: string;
  sourceName: string;
  previewUrl: string;
  attachments: ComposerDraftAttachmentView[];
};

export type PostMediaView = {
  objectId: string;
  kind: 'image' | 'video' | null;
  statusLabel?: string | null;
  extraAttachmentCount: number;
  state: 'loading' | 'ready';
  metaMime?: string | null;
  metaBytesLabel?: string | null;
  imagePreviewSrc?: string | null;
  imageGalleryItems?: Array<{
    hash: string;
    src: string | null;
    mime: string;
  }>;
  currentImageIndex?: number;
  videoPosterPreviewSrc?: string | null;
  videoPlaybackSrc?: string | null;
  videoUnsupportedOnClient: boolean;
  videoProps?: React.VideoHTMLAttributes<HTMLVideoElement>;
  // どの source を正本とし、どの community node capability（media_cache 等）経由で
  // 観測したか。media cache 由来の通報ルーティングに使う。
  provenance?: ContentProvenance;
};

export type ReferencedAuthorMeta = {
  pubkey: string;
  label: string;
  picture?: string | null;
};

// Resolved author info for rendering a mention hover card (in the composer
// suggestion list and in rendered posts).
export type MentionAuthorView = {
  pubkey: string;
  label: string;
  displayName?: string | null;
  name?: string | null;
  aboutPreview?: string | null;
  picture?: string | null;
};

// A candidate offered while typing `@` in the composer.
export type MentionCandidate = {
  pubkey: string;
  label: string;
  displayName?: string | null;
  name?: string | null;
  about?: string | null;
  picture?: string | null;
};

export type PostCardView = {
  post: PostView;
  context: 'timeline' | 'thread';
  authorLabel: string;
  authorPicture?: string | null;
  relationshipLabel: string | null;
  audienceChipLabel?: string | null;
  threadTargetId: string;
  threadTopicId?: string | null;
  canReply?: boolean;
  canRepost?: boolean;
  media: PostMediaView;
  repostSourceAuthor?: ReferencedAuthorMeta | null;
  replyParentAuthor?: ReferencedAuthorMeta | null;
  suppressReplyPreview?: boolean;
  mentionAuthors?: Record<string, MentionAuthorView>;
  // 正本（通常は author_docs）と、index / moderation / cache 等の観測経路を分離して保持する。
  // 通報ルーティング（#310）・content details・default node boundary 説明に使う。
  provenance?: ContentProvenance;
};

export type ThreadPanelState = {
  selectedThreadId: string | null;
  summary: string;
  emptyCopy: string;
};

export type AuthorRelationshipSummary = {
  label: string | null;
  following: boolean;
  followedBy: boolean;
  mutual: boolean;
  friendOfFriend: boolean;
  muted: boolean;
  viaPubkeys: string[];
  isSelf: boolean;
  canFollow: boolean;
  followActionLabel: 'Follow' | 'Unfollow';
  muteActionLabel: 'Mute' | 'Unmute';
};

export type AuthorDetailView = {
  author: AuthorSocialView | null;
  displayLabel: string;
  pictureSrc?: string | null;
  summary: AuthorRelationshipSummary | null;
  canMessage?: boolean;
  authorError?: string | null;
  // profile の canonical source は author_docs。community node はあくまで観測経路であり
  // truth source ではないことを provenance で表す。
  provenance?: ContentProvenance;
};
