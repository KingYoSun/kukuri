import { describe, expect, it } from 'vitest';

import type {
  CommunityNodeConfig,
  CommunityNodeManifest,
  CommunityNodeNodeStatus,
} from './types';
import { eligibleCommunityIndexNodes, resolveCommunityIndexNodeBaseUrl } from './communityIndex';

const manifest = (available: string[]): CommunityNodeManifest => ({
  node_id: 'node',
  node_name: 'node',
  node_role: 'community-node',
  server_name: 'node',
  manifest_version: 'v1',
  capability_scope: { available_enabled: available, planned_enabled: [] },
  authority_scope: { applies_to: ['this_node'], does_not_apply_to: [] },
  p2p_boundary: {
    identity_authority: false,
    profile_canonical_store: false,
    social_graph_canonical_store: false,
    content_truth_source: false,
    network_wide_authority: false,
  },
  abuse_contact: '',
  report_endpoint: '',
  terms_url: '',
  privacy_url: '',
  moderation_policy_url: '',
});

const status = (baseUrl: string, ready: boolean): CommunityNodeNodeStatus => ({
  base_url: baseUrl,
  auto_approve: false,
  auth_state: { authenticated: ready, expires_at: null },
  consent_state: ready ? { all_required_accepted: true, items: [] } : null,
  resolved_urls: null,
  last_error: null,
  invite_code_saved: false,
  admission_rejection: null,
  session_phase: ready ? 'ready' : 'idle',
  retry_after: null,
  restart_required: false,
});

describe('community index node eligibility', () => {
  it('requires configured, authenticated, consented, available capability', () => {
    const config: CommunityNodeConfig = {
      nodes: [{ base_url: 'https://a', auto_approve: false, resolved_urls: null }],
    };
    expect(
      eligibleCommunityIndexNodes(config, [status('https://a', true)], {
        'https://a': { status: 'ok', manifest: manifest(['community_index']) },
      })
    ).toEqual(['https://a']);
    expect(
      eligibleCommunityIndexNodes(config, [status('https://a', false)], {
        'https://a': { status: 'ok', manifest: manifest(['community_index']) },
      })
    ).toEqual([]);
    expect(
      eligibleCommunityIndexNodes(config, [status('https://a', true)], {
        'https://a': { status: 'ok', manifest: manifest([]) },
      })
    ).toEqual([]);
  });

  it('preserves an eligible choice and otherwise selects the first configured node', () => {
    expect(resolveCommunityIndexNodeBaseUrl('https://b', ['https://a', 'https://b'])).toBe(
      'https://b'
    );
    expect(resolveCommunityIndexNodeBaseUrl('https://missing', ['https://a', 'https://b'])).toBe(
      'https://a'
    );
    expect(resolveCommunityIndexNodeBaseUrl('https://missing', [])).toBeNull();
  });
});
