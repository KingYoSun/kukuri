import { useMemo } from 'react';

import type {
  AttachmentView,
  BookmarkedCustomReactionView,
  DirectMessageMessageView,
  NotificationView,
  PostView,
  Profile,
  RecentReactionView,
} from '@/lib/api';

import {
  logMediaDebug,
  selectPrimaryImage,
  selectPrimaryImageAttachment,
  selectVideoManifest,
  selectVideoManifestAttachment,
  selectVideoPoster,
  selectVideoPosterAttachment,
} from '@/shell/media';
import type { DesktopShellState } from '@/shell/store';

type UsePreviewableMediaAttachmentsArgs = {
  activeTimeline: PostView[];
  activePublicTimeline: PostView[];
  profileTimeline: PostView[];
  selectedAuthorTimeline: PostView[];
  thread: PostView[];
  selectedDirectMessageTimeline: DirectMessageMessageView[];
  ownedReactionAssets: DesktopShellState['ownedReactionAssets'];
  bookmarkedReactionAssets: BookmarkedCustomReactionView[];
  recentReactions: RecentReactionView[];
  localProfile: Profile | null;
  knownAuthorsByPubkey: DesktopShellState['knownAuthorsByPubkey'];
  notifications: NotificationView[];
};

export function usePreviewableMediaAttachments({
  activeTimeline,
  activePublicTimeline,
  profileTimeline,
  selectedAuthorTimeline,
  thread,
  selectedDirectMessageTimeline,
  ownedReactionAssets,
  bookmarkedReactionAssets,
  recentReactions,
  localProfile,
  knownAuthorsByPubkey,
  notifications,
}: UsePreviewableMediaAttachmentsArgs): AttachmentView[] {
  return useMemo(() => {
    const attachments = new Map<string, AttachmentView>();

    const tryAddAttachment = (attachment: AttachmentView | null) => {
      if (!attachment) {
        return;
      }
      const hash = attachment.hash.trim();
      const mime = attachment.mime.trim();
      if (!hash || !mime) {
        logMediaDebug('warn', 'remote media metadata skipped', {
          hash: attachment.hash || null,
          mime: attachment.mime || null,
          role: attachment.role,
          status: attachment.status,
        });
        return;
      }
      attachments.set(hash, {
        ...attachment,
        hash,
        mime,
      });
    };

    for (const post of [
      ...activeTimeline,
      ...activePublicTimeline,
      ...profileTimeline,
      ...selectedAuthorTimeline,
      ...thread,
    ]) {
      if (post.author_picture_asset) {
        tryAddAttachment({
          hash: post.author_picture_asset.hash,
          mime: post.author_picture_asset.mime,
          bytes: post.author_picture_asset.bytes,
          role: post.author_picture_asset.role,
          status: 'Available',
        });
      }
      for (const attachment of [
        selectPrimaryImage(post),
        selectVideoPoster(post),
        selectVideoManifest(post),
      ]) {
        tryAddAttachment(attachment);
      }
      for (const reaction of post.reaction_summary ?? []) {
        if (!reaction.custom_asset) {
          continue;
        }
        tryAddAttachment({
          hash: reaction.custom_asset.blob_hash,
          mime: reaction.custom_asset.mime,
          bytes: reaction.custom_asset.bytes,
          role: 'image_original',
          status: 'Available',
        });
      }
    }

    for (const message of selectedDirectMessageTimeline) {
      for (const attachment of [
        selectPrimaryImageAttachment(message.attachments),
        selectVideoPosterAttachment(message.attachments),
        selectVideoManifestAttachment(message.attachments),
      ]) {
        tryAddAttachment(attachment);
      }
    }

    for (const asset of [...ownedReactionAssets, ...bookmarkedReactionAssets]) {
      tryAddAttachment({
        hash: asset.blob_hash,
        mime: asset.mime,
        bytes: asset.bytes,
        role: 'image_original',
        status: 'Available',
      });
    }

    for (const reaction of recentReactions) {
      if (!reaction.custom_asset) {
        continue;
      }
      tryAddAttachment({
        hash: reaction.custom_asset.blob_hash,
        mime: reaction.custom_asset.mime,
        bytes: reaction.custom_asset.bytes,
        role: 'image_original',
        status: 'Available',
      });
    }

    for (const pictureAsset of [
      localProfile?.picture_asset ?? null,
      ...Object.values(knownAuthorsByPubkey).map((author) => author.picture_asset ?? null),
      ...notifications.map((notification) => notification.actor_picture_asset ?? null),
    ]) {
      tryAddAttachment(
        pictureAsset
          ? {
              hash: pictureAsset.hash,
              mime: pictureAsset.mime,
              bytes: pictureAsset.bytes,
              role: pictureAsset.role,
              status: 'Available',
            }
          : null
      );
    }

    return [...attachments.values()];
  }, [
    activePublicTimeline,
    activeTimeline,
    bookmarkedReactionAssets,
    knownAuthorsByPubkey,
    localProfile?.picture_asset,
    notifications,
    ownedReactionAssets,
    profileTimeline,
    recentReactions,
    selectedDirectMessageTimeline,
    selectedAuthorTimeline,
    thread,
  ]);
}
