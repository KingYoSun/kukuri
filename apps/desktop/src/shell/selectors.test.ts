import { describe, expect, it } from 'vitest';

import type { CommunityNodeNodeStatus } from '../lib/api/types';
import { mergeCommunityNodeStatus } from './selectors';

function baseStatus(
  overrides: Partial<CommunityNodeNodeStatus> = {}
): CommunityNodeNodeStatus {
  return {
    base_url: 'https://node.example',
    auth_state: { authenticated: true },
    restart_required: false,
    ...overrides,
  };
}

describe('mergeCommunityNodeStatus', () => {
  it('clears last_error when the node recovers (next reports null)', () => {
    const previous = baseStatus({ last_error: 'community node timeout' });
    const next = baseStatus({ last_error: null });

    const merged = mergeCommunityNodeStatus(previous, next);

    expect(merged.last_error).toBeNull();
  });

  it('keeps a fresh last_error while the node is still failing', () => {
    const previous = baseStatus({ last_error: 'old failure' });
    const next = baseStatus({ last_error: 'new failure' });

    const merged = mergeCommunityNodeStatus(previous, next);

    expect(merged.last_error).toBe('new failure');
  });

  it('preserves config-ish fallbacks from the previous status', () => {
    const resolvedUrls = {
      public_base_url: 'https://node.example',
      connectivity_urls: ['https://node.example/connect'],
    };
    const previous = baseStatus({
      auto_approve: true,
      resolved_urls: resolvedUrls,
      last_error: 'old failure',
    });
    const next = baseStatus({ last_error: null });

    const merged = mergeCommunityNodeStatus(previous, next);

    expect(merged.auto_approve).toBe(true);
    expect(merged.resolved_urls).toEqual(resolvedUrls);
    expect(merged.last_error).toBeNull();
  });
});
