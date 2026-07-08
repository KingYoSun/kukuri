import type {
  DirectMessageConversationView,
  DirectMessageMessageView,
  DirectMessageStatusView,
  TimelineCursor,
} from '@/lib/api';

import { type DraftMediaItem } from '@/shell/slices/shared';

/// ダイレクトメッセージ(WP-H6 PR3 のドメインスライス)。
export type DirectMessagesSliceState = {
  directMessagePaneOpen: boolean;
  selectedDirectMessagePeerPubkey: string | null;
  directMessages: DirectMessageConversationView[];
  directMessageTimelineByPeer: Record<string, DirectMessageMessageView[]>;
  directMessageTimelineNextCursorByPeer: Record<string, TimelineCursor | null>;
  directMessageTimelineLoadingMoreByPeer: Record<string, boolean>;
  directMessageStatusByPeer: Record<string, DirectMessageStatusView>;
  directMessageComposer: string;
  directMessageDraftMediaItems: DraftMediaItem[];
  directMessageAttachmentInputKey: number;
  directMessageError: string | null;
  directMessageSending: boolean;
};

export function createInitialDirectMessagesSlice(): DirectMessagesSliceState {
  return {
    directMessagePaneOpen: false,
    selectedDirectMessagePeerPubkey: null,
    directMessages: [],
    directMessageTimelineByPeer: {},
    directMessageTimelineNextCursorByPeer: {},
    directMessageTimelineLoadingMoreByPeer: {},
    directMessageStatusByPeer: {},
    directMessageComposer: '',
    directMessageDraftMediaItems: [],
    directMessageAttachmentInputKey: 0,
    directMessageError: null,
    directMessageSending: false,
  };
}
