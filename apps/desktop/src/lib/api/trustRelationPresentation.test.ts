import { describe, expect, test } from 'vitest';

import { InvokeError } from './invoke/error';
import { trustRelationUnavailableReason } from './trustRelationPresentation';

describe('trustRelationUnavailableReason', () => {
  test.each([
    ['TRUST_READ_NOT_CONFIGURED', 'trust_not_configured'],
    ['TRUST_READ_NOT_ACTIVATED', 'trust_not_activated'],
    ['RELATION_NOT_FOUND', 'relation_unavailable'],
    // #705: 認証・同意の未達は索引画面と同じ安定理由で案内する。
    ['AUTH_REQUIRED', 'auth_required'],
    ['CONSENT_REQUIRED', 'consent_required'],
    ['SOMETHING_ELSE', 'other'],
  ] as const)('maps %s without inferring an opt-out cause', (code, expected) => {
    expect(trustRelationUnavailableReason(new InvokeError(code, 'server detail'))).toBe(expected);
  });

  test('falls back to the http status for authentication and consent failures', () => {
    expect(trustRelationUnavailableReason(new InvokeError('UNKNOWN', 'x', 401))).toBe('auth_required');
    expect(trustRelationUnavailableReason(new InvokeError('UNKNOWN', 'x', 403))).toBe('consent_required');
  });
});
