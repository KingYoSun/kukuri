import { describe, expect, it } from 'vitest';

import type { AuthorTrustGate, PostView } from '@/lib/api';

import { resolvePostTrustGate } from './authorTrustGates';

// #1061: 投稿の著者と引用元の著者の両方に折りたたみを適用する（ADR 0022 の repost 非表示と同じ範囲）。
function gate(overrides: Partial<AuthorTrustGate> = {}): AuthorTrustGate {
  return {
    author_pubkey: 'author',
    hidden: true,
    node_base_url: 'https://node.example',
    reasons: ['related_users_block_or_mute'],
    expires_at: null,
    always_visible: false,
    ...overrides,
  };
}

function post(overrides: Partial<PostView> = {}): PostView {
  return {
    object_id: 'post-1',
    author_pubkey: 'author',
    ...overrides,
  } as PostView;
}

describe('resolvePostTrustGate', () => {
  it('collapses a post whose author is hidden', () => {
    const resolved = resolvePostTrustGate(post(), { author: gate() });
    expect(resolved).toEqual({
      authorPubkey: 'author',
      nodeBaseUrl: 'https://node.example',
      reasons: ['related_users_block_or_mute'],
      fromRepostSource: false,
    });
  });

  it('collapses a repost whose source author is hidden', () => {
    const resolved = resolvePostTrustGate(
      post({ repost_of: { source_author_pubkey: 'source' } } as Partial<PostView>),
      { source: gate({ author_pubkey: 'source' }) }
    );
    expect(resolved?.authorPubkey).toBe('source');
    expect(resolved?.fromRepostSource).toBe(true);
  });

  it('does not collapse an unevaluated or exempt author', () => {
    expect(resolvePostTrustGate(post(), {})).toBeNull();
    expect(
      resolvePostTrustGate(post(), { author: gate({ hidden: false, always_visible: true }) })
    ).toBeNull();
  });
});
