import i18n from '@/i18n';
import type { AuthorSocialView, IndexEntryView } from '@/lib/api';
import { authorDisplayLabel, resolveProfilePictureSrc } from '@/shell/presentation';

import type { PostCardView } from './types';

export type CommunityIndexOperation = 'search' | 'discovery' | 'recommendations';

type CommunityIndexPostCardViewOptions = {
  nodeBaseUrl: string;
  operation: CommunityIndexOperation;
  topicId: string | null;
  knownAuthor: AuthorSocialView | null;
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
  const topicId =
    entry.scope_kind === 'public_topic' ? entry.scope_id : options.topicId?.trim() || null;
  const channelId = topicId && entry.scope_kind === 'private_channel' ? entry.scope_id : null;
  const authorLabel = authorDisplayLabel(
    entry.author_pubkey,
    knownAuthor?.display_name,
    knownAuthor?.name
  );
  const audience = audienceLabel(entry);

  return {
    post: {
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
      content_status: 'Available',
      attachments: [],
      created_at: entry.created_at,
      reply_to: null,
      reply_preview: null,
      root_id: null,
      object_kind: 'post',
      published_topic_id: topicId,
      origin_topic_id: null,
      repost_of: null,
      repost_commentary: null,
      is_threadable: topicId !== null,
      channel_id: channelId,
      audience_label: audience,
      reaction_summary: [],
      my_reactions: [],
    },
    context: 'timeline',
    authorLabel,
    authorPicture: resolveProfilePictureSrc(knownAuthor, options.mediaObjectUrls),
    relationshipLabel: null,
    audienceChipLabel: audience,
    threadTargetId: entry.object_id,
    threadTopicId: topicId,
    canReply: topicId !== null,
    canRepost: entry.scope_kind === 'public_topic',
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
