import { beforeEach, describe, expect, test } from 'vitest';

import i18n from '@/i18n';
import type { AuthorSocialView, IndexEntryView } from '@/lib/api';

import { communityIndexPostCardView } from './communityIndexPostCardView';

const entry: IndexEntryView = {
  scope_kind: 'public_topic',
  scope_id: 'kukuri:topic:rust',
  object_id: 'post-1',
  author_pubkey: 'a'.repeat(64),
  text: 'indexed text\nderived-tag',
  created_at: 1_700_000_000,
};

const knownAuthor: AuthorSocialView = {
  author_pubkey: entry.author_pubkey,
  name: 'alice',
  display_name: 'Alice',
  about: null,
  picture: 'https://example.test/alice.png',
  picture_asset: null,
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

beforeEach(async () => {
  await i18n.changeLanguage('en');
});

describe('communityIndexPostCardView', () => {
  test('maps only index-provided post data and known author presentation', () => {
    const view = communityIndexPostCardView(entry, {
      nodeBaseUrl: 'https://node.example',
      operation: 'search',
      topicId: null,
      knownAuthor,
      mediaObjectUrls: {},
    });

    expect(view.post).toMatchObject({
      object_id: entry.object_id,
      envelope_id: '',
      author_pubkey: entry.author_pubkey,
      content: entry.text,
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
    expect(view.authorPicture).toBe('https://example.test/alice.png');
    expect(view.audienceChipLabel).toBe('Public');
    expect(view.threadTopicId).toBe(entry.scope_id);
    expect(view.canReply).toBe(true);
    expect(view.canRepost).toBe(true);
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
    expect(view.authorLabel).toBe('Unknown author');
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

  test('restores a private channel interaction context only when its parent topic is known', () => {
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

    expect(view.threadTopicId).toBe('kukuri:topic:rust');
    expect(view.post).toMatchObject({
      published_topic_id: 'kukuri:topic:rust',
      channel_id: 'channel-1',
      is_threadable: true,
    });
    expect(view.canReply).toBe(true);
    expect(view.canRepost).toBe(false);
  });
});
