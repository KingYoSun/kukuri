import { describe, expect, test } from 'vitest';

import type { ContentAdvisory, PostView } from '@/lib/api';

import {
  advisorySubjectKey,
  postAdvisorySubjects,
  resolvePostAdvisory,
  type TimelineContentAdvisoryIndex,
} from './contentAdvisories';

const IMAGE_HASH = 'A'.repeat(64);
const QUOTE_HASH = 'b'.repeat(64);

function advisory(overrides: Partial<ContentAdvisory>): ContentAdvisory {
  return {
    issuer_node_id: 'd'.repeat(64),
    subject_kind: 'blob_cid',
    subject_id: IMAGE_HASH.toLowerCase(),
    category: 'nsfw',
    label: 'adult',
    confidence: 84,
    signal_id: 'signal-1',
    basis: 'classifier_score',
    ...overrides,
  };
}

function post(): PostView {
  return {
    object_id: 'post-1',
    attachments: [
      { hash: IMAGE_HASH, mime: 'image/png', bytes: 1, role: 'image_original', status: 'Available' },
    ],
    repost_of: {
      source_object_id: 'quote-source',
      source_topic_id: 'kukuri:topic:general',
      source_author_pubkey: 'b'.repeat(64),
      source_object_kind: 'post',
      content: 'quote',
      attachments: [
        { hash: QUOTE_HASH, mime: 'image/png', bytes: 1, role: 'image_original', status: 'Available' },
      ],
    },
    reply_preview: {
      object_id: 'reply-parent',
      topic: 'kukuri:topic:general',
      author: { pubkey: 'c'.repeat(64) },
      content: 'parent',
      attachments: [],
    },
    content_labels: [],
  } as unknown as PostView;
}

const INACTIVE = { active: false, settled: {} };

describe('content advisory subjects', () => {
  test('collect the post, its attachments, the quote source and the reply parent', () => {
    expect(postAdvisorySubjects(post())).toEqual([
      { kind: 'post_id', id: 'post-1' },
      { kind: 'blob_cid', id: IMAGE_HASH },
      { kind: 'post_id', id: 'quote-source' },
      { kind: 'blob_cid', id: QUOTE_HASH },
      { kind: 'post_id', id: 'reply-parent' },
    ]);
  });

  test('normalize blob hashes to lowercase keys', () => {
    expect(advisorySubjectKey('blob_cid', ` ${IMAGE_HASH} `)).toBe(
      `blob_cid:${IMAGE_HASH.toLowerCase()}`
    );
    expect(advisorySubjectKey('post_id', ' Post-1 ')).toBe('post_id:Post-1');
  });
});

describe('resolvePostAdvisory', () => {
  test('prefers a blob advisory over a post advisory', () => {
    const index: TimelineContentAdvisoryIndex = {
      'post_id:post-1': [
        {
          advisory: advisory({ subject_kind: 'post_id', subject_id: 'post-1', label: 'sensitive' }),
          nodeBaseUrl: 'n',
        },
      ],
      [advisorySubjectKey('blob_cid', IMAGE_HASH)]: [{ advisory: advisory({}), nodeBaseUrl: 'n' }],
    };
    expect(resolvePostAdvisory(post(), index, INACTIVE).advisory?.advisory.subject_kind).toBe(
      'blob_cid'
    );
  });

  test('uses advisories on the quote source and ignores unknown labels', () => {
    const index: TimelineContentAdvisoryIndex = {
      'post_id:quote-source': [
        {
          advisory: advisory({
            subject_kind: 'post_id',
            subject_id: 'quote-source',
            label: 'violent',
          }),
          nodeBaseUrl: 'n',
        },
      ],
    };
    expect(resolvePostAdvisory(post(), index, INACTIVE).advisory).toBeNull();
    index['post_id:quote-source'].push({
      advisory: advisory({
        subject_kind: 'post_id',
        subject_id: 'quote-source',
        label: 'sensitive',
      }),
      nodeBaseUrl: 'n',
    });
    expect(resolvePostAdvisory(post(), index, INACTIVE).advisory?.advisory.label).toBe(
      'sensitive'
    );
  });

  test('is pending while the lookup is active and a subject is not settled', () => {
    const partial = { active: true, settled: { 'post_id:post-1': true as const } };
    expect(resolvePostAdvisory(post(), {}, partial).pending).toBe(true);
    const settled = Object.fromEntries(
      postAdvisorySubjects(post()).map((subject) => [
        advisorySubjectKey(subject.kind, subject.id),
        true as const,
      ])
    );
    expect(resolvePostAdvisory(post(), {}, { active: true, settled }).pending).toBe(false);
    expect(resolvePostAdvisory(post(), {}, INACTIVE).pending).toBe(false);
  });

  test('a confirmed advisory wins over remaining pending subjects', () => {
    const index: TimelineContentAdvisoryIndex = {
      [advisorySubjectKey('blob_cid', IMAGE_HASH)]: [{ advisory: advisory({}), nodeBaseUrl: 'n' }],
    };
    const state = resolvePostAdvisory(post(), index, { active: true, settled: {} });
    expect(state.advisory).not.toBeNull();
    expect(state.pending).toBe(false);
  });

  // ADR 0046 §6.1: advisory は署名済み `content_labels` へ書き戻さない。
  test('content_advisories_are_separate_from_signed_content_labels', () => {
    const target = post();
    const index: TimelineContentAdvisoryIndex = {
      [advisorySubjectKey('blob_cid', IMAGE_HASH)]: [{ advisory: advisory({}), nodeBaseUrl: 'n' }],
    };
    resolvePostAdvisory(target, index, INACTIVE);
    expect(target.content_labels).toEqual([]);
  });
});
