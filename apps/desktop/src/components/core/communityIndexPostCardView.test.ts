import { beforeEach, describe, expect, test } from 'vitest';

import i18n from '@/i18n';
import type {
  AttachmentView,
  AuthorSocialView,
  CommunityIndexResolvedPostView,
  IndexEntryView,
  PostView,
} from '@/lib/api';

import { communityIndexPostCardView } from './communityIndexPostCardView';

const entry: IndexEntryView = {
  scope_kind: 'public_topic',
  scope_id: 'kukuri:topic:rust',
  object_id: 'post-1',
  author_pubkey: 'a'.repeat(64),
  text: 'indexed text\nderived-tag',
  created_at: 1_700_000_000,
  content_advisories: [],
};

const knownAuthor: AuthorSocialView = {
  author_pubkey: entry.author_pubkey,
  name: 'alice',
  display_name: 'Alice',
  about: null,
  picture_asset: {
    hash: 'avatar-hash',
    mime: 'image/png',
    bytes: 42,
    role: 'profile_avatar',
  },
  updated_at: null,
  following: true,
  followed_by: false,
  mutual: false,
  friend_of_friend: false,
  friend_of_friend_via_pubkeys: [],
  provenance: null,
  muted: false,
  blocking: false,
  blocked_by: false,
};

function resolvedEntry(content: string): CommunityIndexResolvedPostView {
  const post: PostView = {
    object_id: entry.object_id,
    envelope_id: 'envelope-1',
    author_pubkey: entry.author_pubkey,
    author_name: 'alice',
    author_display_name: 'Alice',
    author_picture_asset: null,
    following: false,
    followed_by: false,
    mutual: false,
    friend_of_friend: false,
    provenance: null,
    withdrawal: null,
    content,
    content_status: 'Available',
    attachments: [],
    content_labels: [],
    created_at: entry.created_at,
    reply_to: null,
    reply_preview: null,
    root_id: entry.object_id,
    object_kind: 'post',
    published_topic_id: entry.scope_id,
    origin_topic_id: entry.scope_id,
    repost_of: null,
    repost_commentary: null,
    is_threadable: true,
    channel_id: null,
    audience_label: 'Public',
    reaction_summary: [],
    my_reactions: [],
  };
  return {
    key: `${entry.scope_kind}:${entry.scope_id}:${entry.object_id}`,
    post,
    capabilities: {
      open_thread: true,
      reply: true,
      repost: true,
      quote_repost: true,
      react: true,
      copy_link: true,
      bookmark: true,
      withdraw: false,
    },
  };
}

const PRIMARY_IMAGE_HASH = 'b'.repeat(64);
const SECOND_IMAGE_HASH = 'c'.repeat(64);

function imageAttachments(): AttachmentView[] {
  return [
    {
      hash: PRIMARY_IMAGE_HASH,
      mime: 'image/png',
      bytes: 2048,
      role: 'image_original',
      status: 'Available',
    },
    {
      hash: SECOND_IMAGE_HASH,
      mime: 'image/png',
      bytes: 4096,
      role: 'image_original',
      status: 'Available',
    },
  ];
}

function resolvedImageEntry(labels: string[] = []): CommunityIndexResolvedPostView {
  const resolved = resolvedEntry('canonical signed content');
  if (!resolved.post) throw new Error('resolved post fixture missing');
  resolved.post.attachments = imageAttachments();
  resolved.post.content_labels = labels;
  return resolved;
}

function imageCardView(
  overrides: {
    mediaObjectUrls?: Record<string, string | null>;
    adultContentEnabled?: boolean;
    labels?: string[];
  } = {}
) {
  return communityIndexPostCardView(entry, {
    nodeBaseUrl: 'https://node.example',
    operation: 'search',
    topicId: null,
    knownAuthor,
    resolutionStatus: 'resolved',
    resolvedEntry: resolvedImageEntry(overrides.labels),
    mediaObjectUrls: overrides.mediaObjectUrls ?? {},
    adultContentEnabled: overrides.adultContentEnabled,
    locale: 'en',
  });
}

beforeEach(async () => {
  await i18n.changeLanguage('en');
});

describe('communityIndexPostCardView', () => {
  test('does not expose node-provided text before canonical post resolution', () => {
    const view = communityIndexPostCardView(entry, {
      nodeBaseUrl: 'https://node.example',
      operation: 'search',
      topicId: null,
      knownAuthor,
      mediaObjectUrls: { 'avatar-hash': 'blob:avatar-hash' },
    });

    expect(view.post.content).not.toContain(entry.text);
  });

  test('maps only resolved canonical post data and known author presentation', () => {
    const canonicalContent = 'canonical signed content';
    const view = communityIndexPostCardView(entry, {
      nodeBaseUrl: 'https://node.example',
      operation: 'search',
      topicId: null,
      knownAuthor,
      resolutionStatus: 'resolved',
      resolvedEntry: resolvedEntry(canonicalContent),
      mediaObjectUrls: { 'avatar-hash': 'blob:avatar-hash' },
    });

    expect(view.post).toMatchObject({
      object_id: entry.object_id,
      envelope_id: 'envelope-1',
      author_pubkey: entry.author_pubkey,
      content: canonicalContent,
      content_status: 'Available',
      created_at: entry.created_at,
      object_kind: 'post',
      attachments: [],
      reaction_summary: [],
      my_reactions: [],
      is_threadable: true,
      published_topic_id: entry.scope_id,
      reply_to: null,
      repost_of: null,
    });
    expect(view.authorLabel).toBe('Alice');
    expect(view.authorPicture).toBe('blob:avatar-hash');
    expect(view.audienceChipLabel).toBe('Public');
    expect(view.threadTopicId).toBe(entry.scope_id);
    expect(view.canReply).toBe(true);
    expect(view.canRepost).toBe(true);
    expect(view.canReact).toBe(true);
    expect(view.media).toMatchObject({ kind: null, state: 'ready', extraAttachmentCount: 0 });
    expect(view.identifierCopy).toEqual({
      postId: entry.object_id,
      authorId: entry.author_pubkey,
    });
    expect(view.reportSubjectKind).toBe('search_result');
    expect(view.allowReadOnlyReport).toBe(true);
    expect(view.provenance).toEqual({
      canonicalSource: 'unknown',
      observedVia: [
        { nodeBaseUrl: 'https://node.example', capability: 'community_index' },
      ],
      responsibleReportTargets: [],
    });
  });

  test('gates an adult-labeled canonical result until adult display is enabled', () => {
    const resolved = resolvedEntry('canonical adult content');
    if (!resolved.post) throw new Error('resolved post fixture missing');
    resolved.post.content_labels = ['adult'];

    const hidden = communityIndexPostCardView(entry, {
      nodeBaseUrl: 'https://node.example',
      operation: 'search',
      topicId: null,
      knownAuthor,
      resolutionStatus: 'resolved',
      resolvedEntry: resolved,
      mediaObjectUrls: {},
      adultContentEnabled: false,
    });
    const visible = communityIndexPostCardView(entry, {
      nodeBaseUrl: 'https://node.example',
      operation: 'search',
      topicId: null,
      knownAuthor,
      resolutionStatus: 'resolved',
      resolvedEntry: resolved,
      mediaObjectUrls: {},
      adultContentEnabled: true,
    });

    expect(hidden.adultContentGated).toBe(true);
    expect(visible.adultContentGated).toBe(false);
    expect(hidden.post.content).not.toContain(entry.text);
  });

  // #1052: 解決済み投稿の添付はローカルの署名済み envelope 由来であり、タイムラインと
  // 同じ builder 規則で組み立てる(node の index text からの推測補完ではない)。
  test('builds resolved attachments and media with the timeline rules', () => {
    const view = imageCardView({
      mediaObjectUrls: {
        [PRIMARY_IMAGE_HASH]: 'blob:primary-image',
        [SECOND_IMAGE_HASH]: 'blob:second-image',
      },
    });

    expect(view.post.attachments).toEqual(imageAttachments());
    expect(view.media).toMatchObject({
      objectId: entry.object_id,
      kind: 'image',
      state: 'ready',
      extraAttachmentCount: 1,
      imagePreviewSrc: 'blob:primary-image',
      metaMime: 'image/png',
    });
    expect(view.media.metaBytesLabel).toBeTruthy();
    expect(view.media.imageGalleryItems?.map((item) => item.hash)).toEqual([
      PRIMARY_IMAGE_HASH,
      SECOND_IMAGE_HASH,
    ]);
    expect(view.media.imageGalleryItems?.map((item) => item.src)).toEqual([
      'blob:primary-image',
      'blob:second-image',
    ]);
    expect(view.media.currentImageIndex).toBe(0);
  });

  test('keeps resolved media loading until its object url settles', () => {
    const view = imageCardView({ mediaObjectUrls: {} });

    expect(view.media).toMatchObject({ kind: 'image', state: 'loading' });
    expect(view.media.imagePreviewSrc).toBeNull();
  });

  test('marks resolved media unavailable once its object url settles empty', () => {
    const view = imageCardView({ mediaObjectUrls: { [PRIMARY_IMAGE_HASH]: null } });

    expect(view.media).toMatchObject({ kind: 'image', state: 'unavailable' });
    expect(view.media.imagePreviewSrc).toBeNull();
  });

  test('gates resolved adult-labeled media without exposing a preview source', () => {
    const view = imageCardView({
      labels: ['adult'],
      adultContentEnabled: false,
      mediaObjectUrls: {
        [PRIMARY_IMAGE_HASH]: 'blob:primary-image',
        [SECOND_IMAGE_HASH]: 'blob:second-image',
      },
    });

    expect(view.adultContentGated).toBe(true);
    expect(view.media).toMatchObject({ kind: 'image', state: 'gated' });
    expect(view.media.imagePreviewSrc).toBeNull();
    expect(view.media.imageGalleryItems).toEqual([]);
  });

  test('renders resolved adult-labeled media once adult display is enabled', () => {
    const view = imageCardView({
      labels: ['adult'],
      adultContentEnabled: true,
      mediaObjectUrls: { [PRIMARY_IMAGE_HASH]: 'blob:primary-image' },
    });

    expect(view.adultContentGated).toBe(false);
    expect(view.media).toMatchObject({
      kind: 'image',
      state: 'ready',
      imagePreviewSrc: 'blob:primary-image',
    });
  });

  test.each([
    ['loading', 'loading'],
    ['failed', 'failed'],
  ] as const)('renders no media for a %s entry', (_label, resolutionStatus) => {
    const view = communityIndexPostCardView(entry, {
      nodeBaseUrl: 'https://node.example',
      operation: 'search',
      topicId: null,
      knownAuthor,
      resolutionStatus,
      resolvedEntry: null,
      mediaObjectUrls: { [PRIMARY_IMAGE_HASH]: 'blob:primary-image' },
      locale: 'en',
    });

    expect(view.post.attachments).toEqual([]);
    expect(view.media).toMatchObject({ kind: null, extraAttachmentCount: 0 });
  });

  test.each([
    ['search', 'search_result', 'community_index'],
    ['discovery', 'search_result', 'community_index'],
    ['recommendations', 'recommendation', 'recommendation'],
  ] as const)('maps %s to its report identity', (operation, subjectKind, capability) => {
    const view = communityIndexPostCardView(entry, {
      nodeBaseUrl: 'https://node.example',
      operation,
      topicId: null,
      knownAuthor: null,
      mediaObjectUrls: {},
    });

    expect(view.reportSubjectKind).toBe(subjectKind);
    expect(view.provenance?.observedVia[0]?.capability).toBe(capability);
    expect(view.authorLabel).toBe('Unknown user');
    expect(view.authorPicture).toBeNull();
  });

  test('localizes private scope without exposing its raw identifier', () => {
    const view = communityIndexPostCardView(
      { ...entry, scope_kind: 'private_channel', scope_id: 'private-channel-secret-id' },
      {
        nodeBaseUrl: 'https://node.example',
        operation: 'search',
        topicId: null,
        knownAuthor: null,
        mediaObjectUrls: {},
      }
    );

    expect(view.audienceChipLabel).toBe('Private channel');
    expect(view.threadTopicId).toBeNull();
    expect(view.post.channel_id).toBeNull();
    expect(JSON.stringify(view)).not.toContain('private-channel-secret-id');
  });

  test('keeps a private channel result read-only until its canonical post is resolved', () => {
    const view = communityIndexPostCardView(
      { ...entry, scope_kind: 'private_channel', scope_id: 'channel-1' },
      {
        nodeBaseUrl: 'https://node.example',
        operation: 'search',
        topicId: 'kukuri:topic:rust',
        knownAuthor: null,
        mediaObjectUrls: {},
      }
    );

    expect(view.threadTopicId).toBeNull();
    expect(view.post).toMatchObject({
      published_topic_id: null,
      channel_id: null,
      is_threadable: false,
    });
    expect(view.canReply).toBe(false);
    expect(view.canRepost).toBe(false);
  });
});
