import { type SyntheticEvent, useCallback, useMemo } from 'react';

import type {
  MentionAuthorView,
  PostCardView,
  ReferencedAuthorMeta,
} from '@/components/core/types';
import type { PostView, ProfileAssetView } from '@/lib/api';
import { contentProvenanceFromView } from '@/lib/api/provenance';
import type { SupportedLocale } from '@/i18n';
import { extractMentions } from '@/lib/internalLinks';
import {
  logMediaDebug,
  mediaElementDebugFields,
  selectPrimaryImage,
  selectVideoManifest,
  selectVideoPoster,
} from '@/shell/media';
import {
  authorDisplayLabel,
  canCreateRepostFromPost,
  formatBytes,
  isQuoteRepost,
  localizeAudienceLabel,
  publishedTopicIdForPost,
  resolveProfilePictureSrc,
  shortPubkey,
  strongestRelationshipLabel,
} from '@/shell/presentation';
import {
  useDesktopShellFieldSetter,
  type DesktopShellState,
} from '@/shell/store';

type UseTimelineViewModelsArgs = {
  activeJoinedChannels: DesktopShellState['joinedChannelsByTopic'][string];
  activeTimeline: PostView[];
  bookmarkedPosts: DesktopShellState['bookmarkedPosts'];
  knownAuthorsByPubkey: DesktopShellState['knownAuthorsByPubkey'];
  localAuthorPubkey: string;
  locale: SupportedLocale;
  localProfile: DesktopShellState['localProfile'];
  mediaObjectUrls: DesktopShellState['mediaObjectUrls'];
  profileTimeline: PostView[];
  replyTarget: PostView | null;
  repostTarget: PostView | null;
  selectedAuthorTimeline: PostView[];
  thread: PostView[];
  unsupportedVideoManifests: DesktopShellState['unsupportedVideoManifests'];
};

export function useTimelineViewModels({
  activeJoinedChannels,
  activeTimeline,
  bookmarkedPosts,
  knownAuthorsByPubkey,
  localAuthorPubkey,
  locale,
  localProfile,
  mediaObjectUrls,
  profileTimeline,
  replyTarget,
  repostTarget,
  selectedAuthorTimeline,
  thread,
  unsupportedVideoManifests,
}: UseTimelineViewModelsArgs) {
  const setUnsupportedVideoManifests = useDesktopShellFieldSetter(
    'unsupportedVideoManifests'
  );
  const buildPostCardView = useCallback(
    (post: PostView, context: 'timeline' | 'thread'): PostCardView => {
      const primaryImage = selectPrimaryImage(post);
      const videoPoster = selectVideoPoster(post);
      const videoManifest = selectVideoManifest(post);
      const imageGalleryItems = post.attachments
        .filter(
          (attachment) =>
            attachment.mime.startsWith('image/') && attachment.role !== 'video_poster'
        )
        .map((attachment) => ({
          hash: attachment.hash,
          src:
            typeof mediaObjectUrls[attachment.hash] === 'string'
              ? mediaObjectUrls[attachment.hash]
              : null,
          mime: attachment.mime,
          provenance: contentProvenanceFromView(attachment.provenance),
        }));
      const mediaKind = primaryImage ? 'image' : videoManifest || videoPoster ? 'video' : null;
      const mediaMetaAttachment =
        mediaKind === 'video' ? videoManifest ?? videoPoster : primaryImage;
      const reservedHashes = new Set<string>();
      if (primaryImage) reservedHashes.add(primaryImage.hash);
      if (videoPoster) reservedHashes.add(videoPoster.hash);
      if (videoManifest) reservedHashes.add(videoManifest.hash);
      const extraAttachmentCount = post.attachments.filter(
        (attachment) => !reservedHashes.has(attachment.hash)
      ).length;
      const imagePreviewSrc =
        primaryImage && typeof mediaObjectUrls[primaryImage.hash] === 'string'
          ? mediaObjectUrls[primaryImage.hash]
          : null;
      const videoPosterPreviewSrc =
        videoPoster && typeof mediaObjectUrls[videoPoster.hash] === 'string'
          ? mediaObjectUrls[videoPoster.hash]
          : null;
      const videoPlaybackSrc =
        videoManifest && typeof mediaObjectUrls[videoManifest.hash] === 'string'
          ? mediaObjectUrls[videoManifest.hash]
          : null;
      const videoUnsupportedOnClient = Boolean(
        videoManifest && unsupportedVideoManifests[videoManifest.hash]
      );
      const logPlaybackEvent =
        (eventName: string) => (event: SyntheticEvent<HTMLVideoElement>) => {
          const video = event.currentTarget;
          logMediaDebug(eventName === 'error' ? 'warn' : 'info', `playback ${eventName}`, {
            manifest_hash: videoManifest?.hash ?? null,
            mime: videoManifest?.mime ?? null,
            post_id: post.object_id,
            poster_hash: videoPoster?.hash ?? null,
            playback_src: videoPlaybackSrc,
            ...mediaElementDebugFields(video),
            video_height: video.videoHeight || null,
            video_width: video.videoWidth || null,
          });
          if (eventName === 'error' && videoManifest) {
            setUnsupportedVideoManifests((current) =>
              current[videoManifest.hash]
                ? current
                : { ...current, [videoManifest.hash]: true }
            );
          }
        };
      const publishedTopicId = publishedTopicIdForPost(post);
      const threadTargetId =
        post.object_kind === 'repost' && !isQuoteRepost(post) && post.repost_of
          ? post.repost_of.root_id ?? post.repost_of.source_object_id
          : post.root_id ?? post.object_id;
      const threadTopicId =
        post.object_kind === 'repost' && !isQuoteRepost(post) && post.repost_of
          ? post.repost_of.source_topic_id
          : publishedTopicId;
      const knownAuthor =
        post.author_pubkey === localAuthorPubkey
          ? localProfile
          : knownAuthorsByPubkey[post.author_pubkey] ?? null;
      const authorPicture = resolveProfilePictureSrc(
        {
          picture: post.author_picture ?? knownAuthor?.picture ?? null,
          picture_asset: post.author_picture_asset ?? knownAuthor?.picture_asset ?? null,
        },
        mediaObjectUrls
      );
      const resolveRefAuthor = (
        pubkey: string,
        label: string,
        picture?: string | null,
        pictureAsset?: ProfileAssetView | null
      ): ReferencedAuthorMeta => {
        const known =
          pubkey === localAuthorPubkey ? localProfile : knownAuthorsByPubkey[pubkey] ?? null;
        return {
          pubkey,
          label,
          picture: resolveProfilePictureSrc(
            {
              picture: picture ?? known?.picture ?? null,
              picture_asset: pictureAsset ?? known?.picture_asset ?? null,
            },
            mediaObjectUrls
          ),
        };
      };
      const repostSource = post.repost_of ?? null;
      const repostSourceAuthor = repostSource
        ? resolveRefAuthor(
            repostSource.source_author_pubkey,
            authorDisplayLabel(
              repostSource.source_author_pubkey,
              repostSource.source_author_display_name,
              repostSource.source_author_name
            ),
            repostSource.source_author_picture,
            repostSource.source_author_picture_asset
          )
        : null;
      const replyPreview = post.reply_preview ?? null;
      const replyParentAuthor = replyPreview
        ? resolveRefAuthor(
            replyPreview.author.pubkey,
            authorDisplayLabel(
              replyPreview.author.pubkey,
              replyPreview.author.display_name,
              replyPreview.author.name
            ),
            replyPreview.author.picture,
            replyPreview.author.picture_asset
          )
        : null;
      const mentionAuthors: Record<string, MentionAuthorView> = {};
      for (const source of [post.content, repostSource?.content, replyPreview?.content]) {
        if (!source) continue;
        for (const { pubkey, label } of extractMentions(source)) {
          if (mentionAuthors[pubkey]) continue;
          const known =
            pubkey === localAuthorPubkey ? localProfile : knownAuthorsByPubkey[pubkey] ?? null;
          if (!known) continue;
          mentionAuthors[pubkey] = {
            pubkey,
            label:
              known.display_name?.trim() ||
              known.name?.trim() ||
              label ||
              shortPubkey(pubkey),
            displayName: known.display_name ?? null,
            name: known.name ?? null,
            aboutPreview: known.about?.slice(0, 50) ?? null,
            picture: resolveProfilePictureSrc(known, mediaObjectUrls),
          };
        }
      }
      return {
        post,
        provenance: contentProvenanceFromView(post.provenance),
        context,
        authorLabel: authorDisplayLabel(
          post.author_pubkey,
          post.author_display_name,
          post.author_name
        ),
        authorPicture,
        relationshipLabel: strongestRelationshipLabel(post),
        audienceChipLabel: post.channel_id
          ? activeJoinedChannels.find((channel) => channel.channel_id === post.channel_id)?.label ??
            localizeAudienceLabel(post.audience_label)
          : localizeAudienceLabel(post.audience_label),
        threadTargetId,
        threadTopicId,
        canReply: post.is_threadable ?? (post.object_kind !== 'repost' || isQuoteRepost(post)),
        canRepost: canCreateRepostFromPost(post),
        repostSourceAuthor,
        replyParentAuthor,
        mentionAuthors,
        suppressReplyPreview: context === 'thread',
        media: {
          objectId: post.object_id,
          kind: mediaKind,
          extraAttachmentCount,
          state:
            mediaKind === 'video'
              ? videoPlaybackSrc || videoPosterPreviewSrc
                ? 'ready'
                : 'loading'
              : mediaKind === 'image'
                ? imagePreviewSrc
                  ? 'ready'
                  : 'loading'
                : 'loading',
          metaMime: mediaMetaAttachment?.mime ?? null,
          metaBytesLabel: mediaMetaAttachment
            ? formatBytes(mediaMetaAttachment.bytes, locale)
            : null,
          imagePreviewSrc,
          imageGalleryItems,
          currentImageIndex: primaryImage
            ? Math.max(
                0,
                imageGalleryItems.findIndex((item) => item.hash === primaryImage.hash)
              )
            : 0,
          videoPosterPreviewSrc,
          videoPlaybackSrc,
          videoReportHash:
            mediaKind === 'video' ? (videoManifest?.hash ?? videoPoster?.hash ?? null) : null,
          videoUnsupportedOnClient,
          provenance: contentProvenanceFromView(mediaMetaAttachment?.provenance),
          videoProps:
            mediaKind === 'video' && videoPlaybackSrc && !videoUnsupportedOnClient
              ? {
                  onCanPlay: logPlaybackEvent('canplay'),
                  onDurationChange: logPlaybackEvent('durationchange'),
                  onError: logPlaybackEvent('error'),
                  onLoadedData: logPlaybackEvent('loadeddata'),
                  onLoadedMetadata: logPlaybackEvent('loadedmetadata'),
                  onLoadStart: logPlaybackEvent('loadstart'),
                  onPlaying: logPlaybackEvent('playing'),
                }
              : undefined,
        },
      };
    },
    [
      activeJoinedChannels,
      knownAuthorsByPubkey,
      localAuthorPubkey,
      localProfile,
      locale,
      mediaObjectUrls,
      setUnsupportedVideoManifests,
      unsupportedVideoManifests,
    ]
  );

  return {
    activeTimelinePostViews: useMemo(
      () => activeTimeline.map((post) => buildPostCardView(post, 'timeline')),
      [activeTimeline, buildPostCardView]
    ),
    bookmarkedTimelinePostViews: useMemo(
      () => bookmarkedPosts.map((item) => buildPostCardView(item.post, 'timeline')),
      [bookmarkedPosts, buildPostCardView]
    ),
    profileTimelinePostViews: useMemo(
      () => profileTimeline.map((post) => buildPostCardView(post, 'timeline')),
      [buildPostCardView, profileTimeline]
    ),
    selectedAuthorTimelinePostViews: useMemo(
      () => selectedAuthorTimeline.map((post) => buildPostCardView(post, 'timeline')),
      [buildPostCardView, selectedAuthorTimeline]
    ),
    threadPostViews: useMemo(
      () => thread.map((post) => buildPostCardView(post, 'thread')),
      [buildPostCardView, thread]
    ),
    composerSourcePreview: useMemo(
      () =>
        replyTarget
          ? buildPostCardView(replyTarget, 'timeline')
          : repostTarget
            ? buildPostCardView(repostTarget, 'timeline')
            : null,
      [buildPostCardView, replyTarget, repostTarget]
    ),
  };
}
