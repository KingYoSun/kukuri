import { renderHook } from '@testing-library/react';
import { describe, expect, test } from 'vitest';

import type { ContentAdvisory, PostView } from '@/lib/api';
import { ADULT_CONTENT_LABEL } from '@/shell/media';
import {
  advisorySubjectKey,
  type TimelineContentAdvisoryIndex,
} from '@/shell/contentAdvisories';

import {
  collectAdultGatedMediaHashes,
  useAdultGatedMediaHashes,
} from './useAdultGatedMediaHashes';

// #1107 / AC-6: 同じ blob を advisory 付きの投稿と advisory なしの投稿が参照する場合の規則。

const SHARED_HASH = 'c'.repeat(64);
const OTHER_HASH = 'e'.repeat(64);

function post(objectId: string, hash: string, contentLabels: PostView['content_labels'] = []) {
  return {
    object_id: objectId,
    envelope_id: `envelope-${objectId}`,
    author_pubkey: 'f'.repeat(64),
    object_kind: 'post',
    content: 'caption',
    content_status: 'Available',
    content_labels: contentLabels,
    attachments: [
      { hash, mime: 'image/png', bytes: 1, role: 'image_original', status: 'Available' },
    ],
    created_at: 1,
    reply_to: null,
    root_id: objectId,
    channel_id: null,
    audience_label: 'Public',
  } as unknown as PostView;
}

function advisory(kind: 'post_id' | 'blob_cid', subjectId: string): ContentAdvisory {
  return {
    issuer_node_id: 'd'.repeat(64),
    subject_kind: kind,
    subject_id: subjectId,
    category: 'nsfw',
    label: 'adult',
    confidence: 84,
    signal_id: 'signal-1107',
    basis: 'classifier_score',
  };
}

function index(entries: ContentAdvisory[]): TimelineContentAdvisoryIndex {
  const result: TimelineContentAdvisoryIndex = {};
  for (const entry of entries) {
    (result[advisorySubjectKey(entry.subject_kind, entry.subject_id)] ??= []).push({
      advisory: entry,
      nodeBaseUrl: 'https://api.kukuri.app',
    });
  }
  return result;
}

const advisoryPost = post('advisory-post', SHARED_HASH);
const plainPost = post('plain-post', SHARED_HASH);
const otherPost = post('other-post', OTHER_HASH);
const advisories = index([advisory('post_id', 'advisory-post')]);

describe('collectAdultGatedMediaHashes', () => {
  test('collects the blobs of advisory and self-labeled posts only', () => {
    const labeled = post('labeled-post', OTHER_HASH, [ADULT_CONTENT_LABEL]);
    expect(
      collectAdultGatedMediaHashes({
        posts: [advisoryPost, plainPost, otherPost],
        timelineContentAdvisories: advisories,
        additionalHashes: [],
      })
    ).toEqual([SHARED_HASH]);
    expect(
      collectAdultGatedMediaHashes({
        posts: [plainPost, labeled],
        timelineContentAdvisories: {},
        additionalHashes: [],
      })
    ).toEqual([OTHER_HASH]);
  });

  test('gates a blob advisory even when no visible post references it', () => {
    expect(
      collectAdultGatedMediaHashes({
        posts: [otherPost],
        timelineContentAdvisories: index([advisory('blob_cid', SHARED_HASH)]),
        additionalHashes: ['a'.repeat(64)],
      })
    ).toEqual(['a'.repeat(64), SHARED_HASH]);
  });
});

describe('useAdultGatedMediaHashes', () => {
  test('keeps a gated blob while display is off even after its source post leaves the view', () => {
    const { result, rerender } = renderHook(
      (props: { posts: PostView[]; adultContentEnabled: boolean }) =>
        useAdultGatedMediaHashes({
          adultContentEnabled: props.adultContentEnabled,
          posts: props.posts,
          timelineContentAdvisories: advisories,
          additionalHashes: [],
        }),
      { initialProps: { posts: [advisoryPost, plainPost], adultContentEnabled: false } }
    );
    expect(result.current).toEqual([SHARED_HASH]);
    const first = result.current;

    // 同じ内容なら同じ参照を返す(取得・破棄の effect を再実行しない)。
    rerender({ posts: [advisoryPost, plainPost, otherPost], adultContentEnabled: false });
    expect(result.current).toBe(first);

    // 可視範囲から advisory 付き投稿が外れても、同じ blob を参照する投稿でゲートを続ける。
    rerender({ posts: [plainPost], adultContentEnabled: false });
    expect(result.current).toEqual([SHARED_HASH]);

    // AC-5: ON では空にし、保持していた集合も捨てる。
    rerender({ posts: [plainPost], adultContentEnabled: true });
    expect(result.current).toEqual([]);
    rerender({ posts: [plainPost], adultContentEnabled: false });
    expect(result.current).toEqual([]);

    // OFF のまま根拠の投稿が現れたら、その時点からゲートする。
    rerender({ posts: [plainPost, advisoryPost], adultContentEnabled: false });
    expect(result.current).toEqual([SHARED_HASH]);
  });
});
