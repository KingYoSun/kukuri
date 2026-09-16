import { renderHook } from '@testing-library/react';
import { describe, expect, test } from 'vitest';

import type { AttachmentView, PostView } from '@/lib/api';

import { usePreviewableMediaAttachments } from './usePreviewableMediaAttachments';

const INDEX_IMAGE_HASH = 'a'.repeat(64);
const TIMELINE_IMAGE_HASH = 'b'.repeat(64);

function imagePost(objectId: string, hash: string, contentLabels: string[] = []): PostView {
  const attachment: AttachmentView = {
    hash,
    mime: 'image/png',
    bytes: 2048,
    role: 'image_original',
    status: 'Available',
  };
  return {
    object_id: objectId,
    envelope_id: `envelope-${objectId}`,
    author_pubkey: 'f'.repeat(64),
    author_name: null,
    author_display_name: null,
    following: false,
    followed_by: false,
    mutual: false,
    friend_of_friend: false,
    object_kind: 'post',
    is_threadable: true,
    content: 'caption',
    content_status: 'Available',
    content_labels: contentLabels,
    attachments: [attachment],
    created_at: 1,
    reply_to: null,
    root_id: objectId,
    channel_id: null,
    audience_label: 'Public',
  } as PostView;
}

function renderAttachments(
  overrides: {
    communityIndexResolvedPosts?: PostView[];
    activeTimeline?: PostView[];
    adultContentEnabled?: boolean;
  } = {}
) {
  const { result } = renderHook(() =>
    usePreviewableMediaAttachments({
      activeTimeline: overrides.activeTimeline ?? [],
      activePublicTimeline: [],
      communityIndexResolvedPosts: overrides.communityIndexResolvedPosts ?? [],
      profileTimeline: [],
      selectedAuthorTimeline: [],
      thread: [],
      selectedDirectMessageTimeline: [],
      ownedReactionAssets: [],
      bookmarkedReactionAssets: [],
      recentReactions: [],
      localProfile: null,
      knownAuthorsByPubkey: {},
      notifications: [],
      adultContentEnabled: overrides.adultContentEnabled ?? false,
    })
  );
  return result.current.map((attachment) => attachment.hash);
}

describe('usePreviewableMediaAttachments', () => {
  // #1052: 「見つける」の解決済み投稿もタイムラインと同じプリフェッチ対象にする。
  test('includes attachments from resolved community index posts', () => {
    expect(
      renderAttachments({
        communityIndexResolvedPosts: [imagePost('index-post', INDEX_IMAGE_HASH)],
        activeTimeline: [imagePost('timeline-post', TIMELINE_IMAGE_HASH)],
      })
    ).toEqual(expect.arrayContaining([INDEX_IMAGE_HASH, TIMELINE_IMAGE_HASH]));
  });

  // #858 / ADR 0046: 表示設定 OFF の間は、新しい経路でも成人向け添付を取得対象にしない。
  test('excludes adult-labeled community index attachments while adult display is off', () => {
    expect(
      renderAttachments({
        communityIndexResolvedPosts: [imagePost('index-post', INDEX_IMAGE_HASH, ['adult'])],
      })
    ).not.toContain(INDEX_IMAGE_HASH);
  });

  test('includes adult-labeled community index attachments once adult display is on', () => {
    expect(
      renderAttachments({
        communityIndexResolvedPosts: [imagePost('index-post', INDEX_IMAGE_HASH, ['adult'])],
        adultContentEnabled: true,
      })
    ).toContain(INDEX_IMAGE_HASH);
  });
});
