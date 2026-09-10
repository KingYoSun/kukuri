import { describe, expect, it } from 'vitest';
import { createInstance } from 'i18next';
import { resources } from '@/i18n';

import { type CommunityNodeManifest } from '@/lib/api';
import { type CommunityNodeManifestEntry } from '@/shell/store';

import { buildCommunityNodeDependencyView } from './communityNodeDependency';

// i18n を初期化せずに構造を検証するため、key をそのまま返す stub。
const t = (key: string, options?: Record<string, unknown>) => options?.value ? `${key} (${options.value})` : key;

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
  it('localizes known and unknown manifest values without merging available, planned or authority scopes', async () => {
    const i18n = createInstance();
    await i18n.init({ resources, lng: 'ja', fallbackLng: 'en', interpolation: { escapeValue: false } });
    const entry: CommunityNodeManifestEntry = { status: 'ok', manifest: manifest({
      capability_scope: { available_enabled: ['auth_consent', 'new_capability'], planned_enabled: ['moderation'] },
    }) };
    const before = JSON.stringify(entry);
    const view = buildCommunityNodeDependencyView(entry, i18n.t.bind(i18n));
    expect(view.diagnostics[4].value).toBe('認証と同意 (auth_consent), 不明な値 (new_capability)');
    expect(view.diagnostics[5].value).toBe('モデレーション (moderation)');
    expect(view.diagnostics[6].value).toBe('このノード (this_node)');
    expect(view.diagnostics[7].value).toContain('kukuri ネットワーク全体 (kukuri_network_as_a_whole)');
    expect(JSON.stringify(entry)).toBe(before);
    const missing = buildCommunityNodeDependencyView({ status: 'error', error: 'unreachable' }, i18n.t.bind(i18n));
    expect(missing.diagnostics).toHaveLength(1);
    expect(missing.manifestError).toBe('エラーが発生しました。 （診断詳細: unreachable）');
    await i18n.changeLanguage('zh-CN');
    const translated = buildCommunityNodeDependencyView(entry, i18n.t.bind(i18n));
    expect(translated.diagnostics[4].value).toContain('认证与同意');
    expect(translated.diagnostics[5].value).toContain('内容审核');
    expect(JSON.stringify(entry)).toBe(before);
  });
  it('shows capability scope, authority scope and role for an ok manifest', () => {
    const entry: CommunityNodeManifestEntry = { status: 'ok', manifest: manifest() };
    const view = buildCommunityNodeDependencyView(entry, t);

    const vals = values(view).join('\n');
    expect(vals).toContain('settings:diagnostics.values.capability.auth_consent (auth_consent), settings:diagnostics.values.capability.iroh_relay (iroh_relay)');
    expect(vals).toContain('moderation');
    expect(vals).toContain('this_node');
    expect(vals).toContain('settings:diagnostics.values.authority.user_identity (user_identity), settings:diagnostics.values.authority.kukuri_network_as_a_whole (kukuri_network_as_a_whole)');
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
    expect(view.manifestError).toBe('settings:diagnostics.error settings:diagnostics.original (boom)');
    expect(view.diagnostics[0].tone).toBe('danger');
  });

  it('treats a missing entry as loading', () => {
    const view = buildCommunityNodeDependencyView(undefined, t);
    expect(view.diagnostics[0].value).toBe('settings:communityNode.dependency.status.loading');
  });
});
