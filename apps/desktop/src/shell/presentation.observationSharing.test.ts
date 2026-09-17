import { describe, expect, it } from 'vitest';

import type { CommunityNodeNodeStatus } from '@/lib/api';

import { communityNodeConsentView } from './presentation';

// #1061: 観測提供の任意文書は CN 設定の専用トグルでだけ同意する。通常の同意ダイアログの
// 一覧・一括受諾には出さない（ADR 0026 §8.5）。
function status(): CommunityNodeNodeStatus {
  return {
    base_url: 'https://node.example',
    auth_state: { authenticated: true },
    local_consent: { records: [], withdrawn_at: null },
    invite_code_saved: false,
    restart_required: false,
  } as CommunityNodeNodeStatus;
}

describe('communityNodeConsentView (#1061)', () => {
  it('omits the observation sharing document from the consent catalog', () => {
    const view = communityNodeConsentView(status(), {
      status: 'ok',
      policies: [
        {
          policy_slug: 'terms',
          policy_version: 1,
          title: 'Terms',
          body_markdown: 'Body text',
          required: true,
          is_current: true,
          reference_translation: false,
          fallback: false,
          material_change: false,
          requires_reconsent: false,
        },
        {
          policy_slug: 'trust_observation_sharing',
          policy_version: 1,
          title: 'Sharing',
          body_markdown: 'Sharing body',
          required: false,
          is_current: true,
          reference_translation: false,
          fallback: false,
          material_change: false,
          requires_reconsent: false,
        },
      ],
    });

    expect(view.policies.map((policy) => policy.policySlug)).toEqual(['terms']);
    // 必須文書の判定は変わらない（未同意なので false）。
    expect(view.allRequiredAccepted).toBe(false);
  });
});
