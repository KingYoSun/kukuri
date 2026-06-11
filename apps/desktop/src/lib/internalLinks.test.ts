import { describe, expect, test } from 'vitest';

import {
  buildChannelAccessPreviewDeepLink,
  parseChannelAccessPreviewDeepLink,
} from './internalLinks';

describe('internal link parsing', () => {
  test('parses channel access preview deep links for known token kinds', () => {
    const reference = parseChannelAccessPreviewDeepLink(
      buildChannelAccessPreviewDeepLink('invite:kukuri:topic:demo:channel-1')
    );

    expect(reference).toMatchObject({
      kind: 'share_token',
      tokenKind: 'invite',
      token: 'invite:kukuri:topic:demo:channel-1',
    });
  });

  test('rejects unsupported or malformed channel access preview deep links', () => {
    expect(parseChannelAccessPreviewDeepLink('https://example.com/access-preview?token=invite:x')).toBeNull();
    expect(parseChannelAccessPreviewDeepLink('kukuri://timeline?token=invite:x')).toBeNull();
    expect(parseChannelAccessPreviewDeepLink('kukuri://access-preview/extra?token=invite:x')).toBeNull();
    expect(parseChannelAccessPreviewDeepLink('kukuri://access-preview?token=invite:x#fragment')).toBeNull();
    expect(parseChannelAccessPreviewDeepLink('kukuri://access-preview')).toBeNull();
    expect(parseChannelAccessPreviewDeepLink('kukuri://access-preview?token=unknown:x')).toBeNull();
    expect(parseChannelAccessPreviewDeepLink('kukuri://access-preview?token=invite:x&token=share:y')).toBeNull();
    expect(parseChannelAccessPreviewDeepLink('kukuri://access-preview?token=invite:x&debug=1')).toBeNull();
  });
});
