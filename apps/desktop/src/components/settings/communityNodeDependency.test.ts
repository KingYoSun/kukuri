import { describe, expect, it } from 'vitest';

import { type CommunityNodeManifest } from '@/lib/api';
import { type CommunityNodeManifestEntry } from '@/shell/store';

import { buildCommunityNodeDependencyView } from './communityNodeDependency';

// i18n を初期化せずに構造を検証するため、key をそのまま返す stub。
const t = (key: string) => key;

function manifest(overrides: Partial<CommunityNodeManifest> = {}): CommunityNodeManifest {
  return {
    node_id: '',
    node_name: 'node.example',
    node_role: 'community-node',
    server_name: 'node.example',
    manifest_version: 'v1',
    capability_scope: {
      available_enabled: ['auth_consent', 'iroh_relay'],
      planned_enabled: ['moderation'],
    },
    authority_scope: {
      applies_to: ['this_node'],
      does_not_apply_to: ['user_identity', 'kukuri_network_as_a_whole'],
    },
    p2p_boundary: {
      identity_authority: false,
      profile_canonical_store: false,
      social_graph_canonical_store: false,
      content_truth_source: false,
      network_wide_authority: false,
    },
    abuse_contact: 'abuse@node.example',
    report_endpoint: 'https://node.example/v1/report',
    terms_url: 'https://node.example/terms',
    privacy_url: 'https://node.example/privacy',
    moderation_policy_url: 'https://node.example/moderation-policy',
    ...overrides,
  };
}

function values(view: ReturnType<typeof buildCommunityNodeDependencyView>): string[] {
  return view.diagnostics.map((item) => item.value);
}

describe('buildCommunityNodeDependencyView', () => {
  it('shows capability scope, authority scope and role for an ok manifest', () => {
    const entry: CommunityNodeManifestEntry = { status: 'ok', manifest: manifest() };
    const view = buildCommunityNodeDependencyView(entry, t);

    const vals = values(view).join('\n');
    expect(vals).toContain('auth_consent, iroh_relay');
    expect(vals).toContain('moderation');
    expect(vals).toContain('this_node');
    expect(vals).toContain('user_identity, kukuri_network_as_a_whole');
    // identity が node-owned ではない説明は常に表示。
    expect(view.boundaryNotes).toContain(
      'settings:communityNode.dependency.boundary.identityNotOwned'
    );
    expect(view.manifestError).toBeNull();
  });

  it('marks a default onboarding node and adds the not-network-authority note', () => {
    const entry: CommunityNodeManifestEntry = {
      status: 'ok',
      manifest: manifest({ node_role: 'default-onboarding-node' }),
    };
    const view = buildCommunityNodeDependencyView(entry, t);
    expect(values(view)).toContain('settings:communityNode.dependency.origin.default');
    expect(view.boundaryNotes).toContain(
      'settings:communityNode.dependency.boundary.defaultNotAuthority'
    );
  });

  it('does not add the default-authority note for a user-added community node', () => {
    const entry: CommunityNodeManifestEntry = {
      status: 'ok',
      manifest: manifest({ node_role: 'community-node' }),
    };
    const view = buildCommunityNodeDependencyView(entry, t);
    expect(values(view)).toContain('settings:communityNode.dependency.origin.userAdded');
    expect(view.boundaryNotes).not.toContain(
      'settings:communityNode.dependency.boundary.defaultNotAuthority'
    );
  });

  it('represents absent manifest without scope rows but keeps boundary note', () => {
    const view = buildCommunityNodeDependencyView({ status: 'absent' }, t);
    // manifestStatus 行のみ。capability/authority は出さない。
    expect(view.diagnostics).toHaveLength(1);
    expect(view.diagnostics[0].value).toBe('settings:communityNode.dependency.status.absent');
    expect(view.boundaryNotes).toContain(
      'settings:communityNode.dependency.boundary.identityNotOwned'
    );
  });

  it('surfaces fetch errors and never falls back to a default node', () => {
    const view = buildCommunityNodeDependencyView(
      { status: 'error', error: 'boom' },
      t
    );
    expect(view.manifestError).toBe('boom');
    expect(view.diagnostics[0].tone).toBe('danger');
  });

  it('treats a missing entry as loading', () => {
    const view = buildCommunityNodeDependencyView(undefined, t);
    expect(view.diagnostics[0].value).toBe('settings:communityNode.dependency.status.loading');
  });
});
