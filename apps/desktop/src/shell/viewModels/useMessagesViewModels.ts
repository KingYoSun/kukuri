import { useMemo } from 'react';

import type { ComposerDraftMediaView } from '@/components/core/types';
import type { SupportedLocale } from '@/i18n';
import {
  authorDisplayLabel,
  formatBytes,
  resolveProfilePictureSrc,
} from '@/shell/presentation';
import type { DesktopShellState } from '@/shell/store';

type UseMessagesViewModelsArgs = {
  directMessageDraftMediaItems: DesktopShellState['directMessageDraftMediaItems'];
  directMessages: DesktopShellState['directMessages'];
  directMessageStatusByPeer: DesktopShellState['directMessageStatusByPeer'];
  directMessageTimelineByPeer: DesktopShellState['directMessageTimelineByPeer'];
  knownAuthorsByPubkey: DesktopShellState['knownAuthorsByPubkey'];
  locale: SupportedLocale;
  localProfile: DesktopShellState['localProfile'];
  mediaObjectUrls: DesktopShellState['mediaObjectUrls'];
  selectedDirectMessagePeerPubkey: DesktopShellState['selectedDirectMessagePeerPubkey'];
  translate: (key: string, options?: Record<string, unknown>) => string;
};

// messages section の projection(Q3 T5 の分割先)。
export function useMessagesViewModels({
  directMessageDraftMediaItems,
  directMessages,
  directMessageStatusByPeer,
  directMessageTimelineByPeer,
  knownAuthorsByPubkey,
  locale,
  localProfile,
  mediaObjectUrls,
  selectedDirectMessagePeerPubkey,
  translate,
}: UseMessagesViewModelsArgs) {
  const selectedDirectMessageConversation =
    directMessages.find(
      (conversation) => conversation.peer_pubkey === selectedDirectMessagePeerPubkey
    ) ?? null;
  const selectedDirectMessageTimeline =
    directMessageTimelineByPeer[selectedDirectMessagePeerPubkey ?? ''] ?? [];
  const selectedDirectMessageStatus =
    directMessageStatusByPeer[selectedDirectMessagePeerPubkey ?? ''] ??
    selectedDirectMessageConversation?.status ??
    null;
  const selectedDirectMessagePeerAuthor = selectedDirectMessageConversation
    ? knownAuthorsByPubkey[selectedDirectMessageConversation.peer_pubkey] ?? null
    : null;
  const selectedDirectMessagePeerLabel = selectedDirectMessageConversation
    ? authorDisplayLabel(
        selectedDirectMessageConversation.peer_pubkey,
        selectedDirectMessageConversation.peer_display_name,
        selectedDirectMessageConversation.peer_name
      )
    : null;
  const selectedDirectMessagePeerPicture = resolveProfilePictureSrc(
    selectedDirectMessagePeerAuthor,
    mediaObjectUrls
  );
  const localDirectMessageAuthorPicture = resolveProfilePictureSrc(
    localProfile,
    mediaObjectUrls
  );

  const directMessageDraftViews = useMemo<ComposerDraftMediaView[]>(
    () =>
      directMessageDraftMediaItems.map((item) => ({
        id: item.id,
        sourceName: item.source_name,
        previewUrl: item.preview_url,
        attachments: item.attachments.map((attachment) => ({
          key: `${attachment.role ?? attachment.mime}-${attachment.file_name ?? item.source_name}`,
          label: attachment.role ?? translate('common:fallbacks.attachment'),
          mime: attachment.mime,
          byteSizeLabel: formatBytes(attachment.byte_size, locale),
        })),
      })),
    [directMessageDraftMediaItems, locale, translate]
  );

  return {
    selectedDirectMessageConversation,
    selectedDirectMessageTimeline,
    selectedDirectMessageStatus,
    selectedDirectMessagePeerAuthor,
    selectedDirectMessagePeerLabel,
    selectedDirectMessagePeerPicture,
    localDirectMessageAuthorPicture,
    directMessageDraftViews,
  };
}
