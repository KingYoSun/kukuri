import { describe, expect, it } from 'vitest';

import type { CommunityNodeManifest } from '@/lib/api/types.generated';
import type { CommunityNodeManifestEntry } from '@/shell/slices/connectivity';

import { buildCommunityNodeDisclosures } from './communityNodeDisclosures';

function manifest(overrides: Partial<CommunityNodeManifest> = {}): CommunityNodeManifest {
  return {
    node_id: 'node-a',
    node_name: 'Example Node',
    node_role: 'community-node',
    server_name: 'node.example',
    manifest_version: 'v1',
    capability_scope: { available_enabled: [], planned_enabled: [] },
    authority_scope: { applies_to: [], does_not_apply_to: [] },
    p2p_boundary: {
      identity_authority: false,
      profile_canonical_store: false,
      social_graph_canonical_store: false,
      content_truth_source: false,
      network_wide_authority: false,
    },
    abuse_contact: '',
    report_endpoint: '',
    terms_url: 'https://node.example/terms',
    privacy_url: 'https://node.example/privacy',
    moderation_policy_url: '',
    external_transmission_url: 'https://node.example/external-transmission',
    abuse_policy_url: 'https://node.example/abuse-policy',
    data_retention_url: 'https://node.example/data-retention',
    ...overrides,
  };
}

describe('buildCommunityNodeDisclosures', () => {
  it('uses only each configured node manifest URLs', () => {
    const entries: Record<string, CommunityNodeManifestEntry> = {
      'https://node.example': { status: 'ok', manifest: manifest() },
    };
    const result = buildCommunityNodeDisclosures(
      { nodes: [{ base_url: 'https://node.example' }] },
      entries
    );

    expect(result[0].links.map((link) => link.href)).toEqual([
      'https://node.example/terms',
      'https://node.example/privacy',
      'https://node.example/external-transmission',
      'https://node.example/abuse-policy',
      'https://node.example/data-retention',
    ]);
  });

  it('does not fall back when the configured node manifest is unavailable', () => {
    const result = buildCommunityNodeDisclosures(
      { nodes: [{ base_url: 'https://third-party.example' }] },
      { 'https://third-party.example': { status: 'error', error: 'offline' } }
    );

    expect(result).toEqual([
      {
        baseUrl: 'https://third-party.example',
        nodeName: null,
        links: [],
        manifestAvailable: false,
      },
    ]);
    expect(JSON.stringify(result)).not.toContain('kukuri.app');
  });
});
