import { describe, expect, test } from 'vitest';

import { InvokeError } from './invoke/error';
import { trustRelationUnavailableReason } from './trustRelationPresentation';

describe('trustRelationUnavailableReason', () => {
  test.each([
    ['TRUST_READ_NOT_CONFIGURED', 'trust_not_configured'],
    ['TRUST_READ_NOT_ACTIVATED', 'trust_not_activated'],
    ['RELATION_NOT_FOUND', 'relation_unavailable'],
    ['AUTH_REQUIRED', 'other'],
  ] as const)('maps %s without inferring an opt-out cause', (code, expected) => {
    expect(trustRelationUnavailableReason(new InvokeError(code, 'server detail'))).toBe(expected);
  });
});
