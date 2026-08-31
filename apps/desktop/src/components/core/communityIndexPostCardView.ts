import i18n from '@/i18n';
import type {
  AuthorSocialView,
  CommunityIndexResolvedPostView,
  IndexEntryView,
} from '@/lib/api';
import { authorDisplayLabel, resolveProfilePictureSrc } from '@/shell/presentation';

import type { PostCardView } from './types';

export type CommunityIndexOperation = 'search' | 'discovery' | 'recommendations';

type CommunityIndexPostCardViewOptions = {
  nodeBaseUrl: string;
  operation: CommunityIndexOperation;
  topicId: string | null;
  knownAuthor: AuthorSocialView | null;
  authorStatus?: 'loading' | 'resolved' | 'failed';
  resolvedEntry?: CommunityIndexResolvedPostView | null;
  mediaObjectUrls: Record<string, string | null>;
};

function audienceLabel(entry: IndexEntryView): string {
  return entry.scope_kind === 'private_channel'
    ? i18n.t('common:audience.privateChannel')
    : i18n.t('common:audience.public');
}

export function communityIndexPostCardView(
  entry: IndexEntryView,
  options: CommunityIndexPostCardViewOptions
): PostCardView {
  const recommendation = options.operation === 'recommendations';
  const capability = recommendation ? 'recommendation' : 'community_index';
  const knownAuthor = options.knownAuthor;
  const resolvedPost = options.resolvedEntry?.post ?? null;
  const capabilities = options.resolvedEntry?.capabilities ?? {
    open_thread: false,
    reply: false,
    repost: false,
    quote_repost: false,
    react: false,
    copy_link: false,
    bookmark: false,
    withdraw: false,
  };
  const topicId = resolvedPost
    ? resolvedPost.published_topic_id?.trim() || resolvedPost.origin_topic_id?.trim() || null
    : entry.scope_kind === 'public_topic'
      ? entry.scope_id
      : options.topicId?.trim() || null;
  const resolvedAuthorDisplayName = resolvedPost?.author_display_name ?? null;
  const resolvedAuthorName = resolvedPost?.author_name ?? null;
  const hasResolvedAuthorName = Boolean(
    resolvedAuthorDisplayName?.trim() || resolvedAuthorName?.trim()
  );
  const authorLabel = knownAuthor || hasResolvedAuthorName
    ? authorDisplayLabel(
        entry.author_pubkey,
        knownAuthor?.display_name ?? resolvedAuthorDisplayName,
        knownAuthor?.name ?? resolvedAuthorName
      )
    : options.authorStatus === 'failed'
      ? i18n.t('common:fallbacks.authorUnavailable')
      : options.authorStatus === 'loading'
        ? i18n.t('common:fallbacks.authorLoading')
        : i18n.t('common:fallbacks.unknownAuthor');
  const audience = audienceLabel(entry);
  const displayPost = resolvedPost
    ? {
        ...resolvedPost,
        author_pubkey: entry.author_pubkey,
        author_name: knownAuthor?.name ?? resolvedPost.author_name ?? null,
        author_display_name:
          knownAuthor?.display_name ?? resolvedPost.author_display_name ?? null,
        author_picture: knownAuthor?.picture ?? resolvedPost.author_picture ?? null,
        author_picture_asset:
          knownAuthor?.picture_asset ?? resolvedPost.author_picture_asset ?? null,
        content: entry.text,
        content_status: 'Available' as const,
        attachments: [],
        created_at: entry.created_at,
        repost_of: null,
        repost_commentary: null,
      }
    : {
        object_id: entry.object_id,
        // IndexEntryView は envelope id を提供しない。identifierCopy でも明示的に除外する。
        envelope_id: '',
        author_pubkey: entry.author_pubkey,
        author_name: knownAuthor?.name ?? null,
        author_display_name: knownAuthor?.display_name ?? null,
        author_picture: knownAuthor?.picture ?? null,
        author_picture_asset: knownAuthor?.picture_asset ?? null,
        following: knownAuthor?.following ?? false,
        followed_by: knownAuthor?.followed_by ?? false,
        mutual: knownAuthor?.mutual ?? false,
        friend_of_friend: knownAuthor?.friend_of_friend ?? false,
        provenance: null,
        withdrawal: null,
        content: entry.text,
        content_status: 'Available' as const,
        attachments: [],
        created_at: entry.created_at,
        reply_to: null,
        reply_preview: null,
        root_id: null,
        object_kind: 'post',
        published_topic_id: null,
        origin_topic_id: null,
        repost_of: null,
        repost_commentary: null,
        is_threadable: false,
        channel_id: null,
        audience_label: audience,
        reaction_summary: [],
        my_reactions: [],
      };

  return {
    post: displayPost,
    actionPost: resolvedPost,
    context: 'timeline',
    authorLabel,
    authorPicture: knownAuthor
      ? resolveProfilePictureSrc(knownAuthor, options.mediaObjectUrls)
      : resolvedPost?.author_picture_asset?.hash &&
          typeof options.mediaObjectUrls[resolvedPost.author_picture_asset.hash] === 'string'
        ? options.mediaObjectUrls[resolvedPost.author_picture_asset.hash]
        : resolvedPost?.author_picture ?? null,
    relationshipLabel: null,
    audienceChipLabel: audience,
    threadTargetId: resolvedPost?.root_id ?? entry.object_id,
    threadTopicId: capabilities.open_thread ? topicId : null,
    canOpenThread: capabilities.open_thread,
    canReply: capabilities.reply,
    canRepost: capabilities.repost || capabilities.quote_repost,
    canReact: capabilities.react,
    media: {
      objectId: entry.object_id,
      kind: null,
      extraAttachmentCount: 0,
      state: 'ready',
      videoUnsupportedOnClient: false,
    },
    provenance: {
      canonicalSource: 'unknown',
      observedVia: [{ nodeBaseUrl: options.nodeBaseUrl, capability }],
      responsibleReportTargets: [],
    },
    identifierCopy: {
      postId: entry.object_id,
      authorId: entry.author_pubkey,
    },
    reportSubjectKind: recommendation ? 'recommendation' : 'search_result',
    allowReadOnlyReport: true,
  };
}
