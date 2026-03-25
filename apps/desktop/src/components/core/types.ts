import type * as React from 'react';

import type { AuthorSocialView, PostView } from '@/lib/api';

export type TopicDiagnosticSummary = {
  topic: string;
  active: boolean;
  removable: boolean;
  connectionLabel: string;
  peerCount: number;
  lastReceivedLabel: string;
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
  statusLabel: string | null;
  extraAttachmentCount: number;
  state: 'loading' | 'ready';
  metaMime?: string | null;
  metaBytesLabel?: string | null;
  imagePreviewSrc?: string | null;
  videoPosterPreviewSrc?: string | null;
  videoPlaybackSrc?: string | null;
  videoUnsupportedOnClient: boolean;
  videoProps?: React.VideoHTMLAttributes<HTMLVideoElement>;
};

export type PostCardView = {
  post: PostView;
  context: 'timeline' | 'thread';
  authorLabel: string;
  relationshipLabel: string | null;
  threadTargetId: string;
  media: PostMediaView;
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
  viaPubkeys: string[];
  isSelf: boolean;
  canFollow: boolean;
  followActionLabel: 'Follow' | 'Unfollow';
};

export type AuthorDetailView = {
  author: AuthorSocialView | null;
  displayLabel: string;
  summary: AuthorRelationshipSummary | null;
  authorError?: string | null;
};
