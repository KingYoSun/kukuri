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
  isAdultLabeledPost,
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
  /// #1052: 「見つける」Column の解決済み投稿。タイムラインと同じ規則でメディアを
  /// プリフェッチする(未解決 entry は attachments を持たないため対象にならない)。
  communityIndexResolvedPosts: PostView[];
  /// #1055: Community Node の content advisory でゲート中の添付 blob hash。表示設定 OFF の間は
  /// どの表示経路から現れてもプリフェッチしない(ADR 0046 §6.2)。advisory は `PostView` に
  /// 現れないため、投稿単位ではなく hash 単位で止める。
  advisoryGatedMediaHashes: string[];
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
  adultContentEnabled: boolean;
};

export function usePreviewableMediaAttachments({
  activeTimeline,
  activePublicTimeline,
  communityIndexResolvedPosts,
  advisoryGatedMediaHashes,
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
  adultContentEnabled,
}: UsePreviewableMediaAttachmentsArgs): AttachmentView[] {
  return useMemo(() => {
    const attachments = new Map<string, AttachmentView>();
    const gatedHashes = new Set(advisoryGatedMediaHashes);

    const tryAddAttachment = (attachment: AttachmentView | null) => {
      if (!attachment) {
        return;
      }
      const hash = attachment.hash.trim();
      const mime = attachment.mime.trim();
      // #1055: advisory でゲート中の hash は取得対象に入れない。
      if (gatedHashes.has(hash)) {
        return;
      }
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
      ...communityIndexResolvedPosts,
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
      // #858: 表示許可前は成人向けラベル付き投稿の添付をプリフェッチ対象に入れない
      // (author avatar とリアクションはラベル対象外)。
      if (adultContentEnabled || !isAdultLabeledPost(post)) {
        for (const attachment of [
          selectPrimaryImage(post),
          selectVideoPoster(post),
          selectVideoManifest(post),
        ]) {
          tryAddAttachment(attachment);
        }
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
    adultContentEnabled,
    advisoryGatedMediaHashes,
    bookmarkedReactionAssets,
    communityIndexResolvedPosts,
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
