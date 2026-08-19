import { describe, expect, it } from 'vitest';

import { type ContentProvenance, unknownProvenance } from './provenance';
import {
  manifestToReportTarget,
  nodeAcceptsReportForCapability,
  planAppealReportRouting,
  planReportRouting,
  resolveReportTargetsFromManifests,
} from './reportRouting';
import { type CommunityNodeManifest } from './types';

function manifest(overrides: Partial<CommunityNodeManifest> = {}): CommunityNodeManifest {
  return {
    node_id: 'node-1',
    node_name: 'node.example',
    node_role: 'community-node',
    server_name: 'node.example',
    manifest_version: 'v1',
    // 本番の語彙(cn-operator の capability key / authority scope)で、索引・モデレーション・
    // 信頼・添付保管を提供中のノード。
    capability_scope: {
      available_enabled: [
        'community_index',
        'moderation',
        'community_local_trust',
        'blob_cache',
        'report_endpoint',
      ],
      planned_enabled: [],
    },
    authority_scope: {
      applies_to: [
        'this_node',
        'communities_indexed_by_this_node',
        'moderation_events_issued_by_this_node',
        'trust_signals_issued_by_this_node',
        'media_cached_by_this_node',
      ],
      does_not_apply_to: ['user_identity'],
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
    privacy_url: '',
    moderation_policy_url: '',
    ...overrides,
  };
}

const indexedProvenance: ContentProvenance = {
  canonicalSource: 'author_docs',
  observedVia: [{ nodeBaseUrl: 'https://node.example', capability: 'community_index' }],
  responsibleReportTargets: [],
};

describe('nodeAcceptsReportForCapability', () => {
  it('accepts when node claims authority and does not disclaim the capability', () => {
    expect(nodeAcceptsReportForCapability(manifest(), 'community_index')).toBe(true);
  });

  it('rejects when authority scope is empty (node claims no responsibility)', () => {
    const m = manifest({ authority_scope: { applies_to: [], does_not_apply_to: [] } });
    expect(nodeAcceptsReportForCapability(m, 'community_index')).toBe(false);
  });

  it('rejects when the capability is explicitly disclaimed', () => {
    const m = manifest({
      authority_scope: {
        applies_to: ['this_node', 'communities_indexed_by_this_node', 'moderation_events_issued_by_this_node'],
        does_not_apply_to: ['moderation'],
      },
    });
    expect(nodeAcceptsReportForCapability(m, 'moderation')).toBe(false);
    expect(nodeAcceptsReportForCapability(m, 'community_index')).toBe(true);
  });

  // #702: 提供中能力と責任範囲の両方を要求する。
  it('rejects capabilities the node does not provide even if it claims this_node', () => {
    const indexOnly = manifest({
      capability_scope: { available_enabled: ['community_index'], planned_enabled: [] },
      authority_scope: { applies_to: ['this_node'], does_not_apply_to: [] },
    });
    expect(nodeAcceptsReportForCapability(indexOnly, 'moderation')).toBe(false);
    expect(nodeAcceptsReportForCapability(indexOnly, 'trust_signal')).toBe(false);
    expect(nodeAcceptsReportForCapability(indexOnly, 'media_cache')).toBe(false);
    // 索引の責任範囲語彙が無ければ索引の通報先にもならない。
    expect(nodeAcceptsReportForCapability(indexOnly, 'community_index')).toBe(false);
    expect(manifestToReportTarget('https://node.example', indexOnly, 'moderation')).toBeNull();
  });

  const indexScoped = (available: string[], applies: string[]) =>
    manifest({
      capability_scope: { available_enabled: available, planned_enabled: [] },
      authority_scope: { applies_to: applies, does_not_apply_to: [] },
    });

  it.each([
    ['community_index', ['community_index'], 'communities_indexed_by_this_node'],
    ['recommendation', ['community_index'], 'communities_indexed_by_this_node'],
    ['moderation', ['moderation'], 'moderation_events_issued_by_this_node'],
    ['trust_signal', ['community_local_trust'], 'trust_signals_issued_by_this_node'],
    ['media_cache', ['blob_cache'], 'media_cached_by_this_node'],
    ['bootstrap_assist', ['bootstrap_assist'], 'this_node'],
    ['relay_assist', ['iroh_relay'], 'this_node'],
    ['relay_assist', ['traffic_relay_fallback'], 'this_node'],
  ] as const)('requires the provided capability and scope for %s', (capability, available, scope) => {
    const applies = scope === 'this_node' ? ['this_node'] : ['this_node', scope];
    // 提供中 + 責任範囲あり → 候補。
    expect(nodeAcceptsReportForCapability(indexScoped([...available], applies), capability)).toBe(true);
    // 責任範囲が this_node だけ → 候補にしない(接続補助系を除く)。
    if (scope !== 'this_node') {
      expect(nodeAcceptsReportForCapability(indexScoped([...available], ['this_node']), capability)).toBe(false);
    }
    // 能力が未提供(失効)→ 候補にしない。
    expect(nodeAcceptsReportForCapability(indexScoped([], applies), capability)).toBe(false);
    // 計画中だけ → 候補にしない。
    const planned = manifest({
      capability_scope: { available_enabled: [], planned_enabled: [...available] },
      authority_scope: { applies_to: applies, does_not_apply_to: [] },
    });
    expect(nodeAcceptsReportForCapability(planned, capability)).toBe(false);
    // 責任範囲語彙の明示的否認 → 候補にしない。
    const disclaimed = manifest({
      capability_scope: { available_enabled: [...available], planned_enabled: [] },
      authority_scope: { applies_to: applies, does_not_apply_to: [scope] },
    });
    expect(nodeAcceptsReportForCapability(disclaimed, capability)).toBe(false);
  });

  it('rejects nodes that claim network-wide authority (P2P boundary invariant)', () => {
    const m = manifest({
      p2p_boundary: {
        identity_authority: false,
        profile_canonical_store: false,
        social_graph_canonical_store: false,
        content_truth_source: false,
        network_wide_authority: true,
      },
    });
    expect(nodeAcceptsReportForCapability(m, 'community_index')).toBe(false);
  });
});

describe('manifestToReportTarget', () => {
  it('builds a target with endpoint, contact, policy and authority scope', () => {
    const target = manifestToReportTarget('https://node.example', manifest(), 'community_index');
    expect(target).toEqual({
      nodeBaseUrl: 'https://node.example',
      nodeId: 'node-1',
      capability: 'community_index',
      reportEndpoint: 'https://node.example/v1/report',
      abuseContact: 'abuse@node.example',
      policyUrl: 'https://node.example/terms',
      authorityScope: [
        'this_node',
        'communities_indexed_by_this_node',
        'moderation_events_issued_by_this_node',
        'trust_signals_issued_by_this_node',
        'media_cached_by_this_node',
      ],
    });
  });

  it('prefers moderation policy url over terms url for policyUrl', () => {
    const target = manifestToReportTarget(
      'https://node.example',
      manifest({ moderation_policy_url: 'https://node.example/moderation' }),
      'moderation',
    );
    expect(target?.policyUrl).toBe('https://node.example/moderation');
  });

  it('returns null when the node has neither endpoint nor abuse contact', () => {
    const m = manifest({ report_endpoint: '', abuse_contact: '' });
    expect(manifestToReportTarget('https://node.example', m, 'community_index')).toBeNull();
  });

  it('returns null when the node does not claim authority', () => {
    const m = manifest({ authority_scope: { applies_to: [], does_not_apply_to: [] } });
    expect(manifestToReportTarget('https://node.example', m, 'community_index')).toBeNull();
  });
});

describe('resolveReportTargetsFromManifests', () => {
  it('derives a report target from observedVia and the fetched manifest', () => {
    const targets = resolveReportTargetsFromManifests(indexedProvenance, {
      'https://node.example': manifest(),
    });
    expect(targets).toHaveLength(1);
    expect(targets[0].nodeBaseUrl).toBe('https://node.example');
    expect(targets[0].capability).toBe('community_index');
    expect(targets[0].reportEndpoint).toBe('https://node.example/v1/report');
  });

  it('never falls back to a default node when provenance is unknown', () => {
    expect(resolveReportTargetsFromManifests(unknownProvenance(), {})).toEqual([]);
    expect(resolveReportTargetsFromManifests(null, { 'https://node.example': manifest() })).toEqual(
      [],
    );
  });

  it('does not synthesize a target when the observed node manifest is missing', () => {
    // 観測したが manifest 未取得 → 通報先を合成しない。
    expect(resolveReportTargetsFromManifests(indexedProvenance, {})).toEqual([]);
  });

  it('does not route to a bridge capability', () => {
    const bridgeProvenance: ContentProvenance = {
      canonicalSource: 'external_bridge',
      observedVia: [{ nodeBaseUrl: 'https://node.example', capability: 'bridge' }],
      responsibleReportTargets: [],
    };
    expect(
      resolveReportTargetsFromManifests(bridgeProvenance, { 'https://node.example': manifest() }),
    ).toEqual([]);
  });

  it('prefers explicit responsibleReportTargets and dedupes by node+capability', () => {
    const provenance: ContentProvenance = {
      canonicalSource: 'author_docs',
      observedVia: [{ nodeBaseUrl: 'https://node.example', capability: 'community_index' }],
      responsibleReportTargets: [
        {
          nodeBaseUrl: 'https://node.example',
          capability: 'community_index',
          reportEndpoint: 'https://node.example/explicit',
        },
      ],
    };
    const targets = resolveReportTargetsFromManifests(provenance, {
      'https://node.example': manifest(),
    });
    // 同じ node+capability なので 1 件。明示 target を優先する。
    expect(targets).toHaveLength(1);
    expect(targets[0].reportEndpoint).toBe('https://node.example/explicit');
  });

  it('resolves multiple distinct nodes/capabilities', () => {
    const provenance: ContentProvenance = {
      canonicalSource: 'community_docs',
      observedVia: [
        { nodeBaseUrl: 'https://index.example', capability: 'community_index' },
        { nodeBaseUrl: 'https://cache.example', capability: 'media_cache' },
      ],
      responsibleReportTargets: [],
    };
    const targets = resolveReportTargetsFromManifests(provenance, {
      'https://index.example': manifest({ node_id: 'idx' }),
      'https://cache.example': manifest({ node_id: 'cache' }),
    });
    expect(targets.map((t) => t.nodeBaseUrl)).toEqual([
      'https://index.example',
      'https://cache.example',
    ]);
    expect(targets.map((t) => t.capability)).toEqual(['community_index', 'media_cache']);
  });
});

describe('planReportRouting', () => {
  it('reports candidates with resolved contact methods', () => {
    const plan = planReportRouting(indexedProvenance, { 'https://node.example': manifest() });
    expect(plan.provenanceUnknown).toBe(false);
    expect(plan.localActionsOnly).toBe(false);
    expect(plan.observedButUnresolved).toBe(false);
    expect(plan.candidates).toHaveLength(1);
    expect(plan.candidates[0].contact).toEqual({
      kind: 'endpoint',
      value: 'https://node.example/v1/report',
    });
  });

  it('falls back to abuse contact when no report endpoint is published', () => {
    const plan = planReportRouting(indexedProvenance, {
      'https://node.example': manifest({ report_endpoint: '' }),
    });
    expect(plan.candidates[0].contact).toEqual({
      kind: 'contact',
      value: 'abuse@node.example',
    });
  });

  it('marks local-actions-only and provenance unknown for unknown provenance', () => {
    const plan = planReportRouting(unknownProvenance(), {});
    expect(plan.provenanceUnknown).toBe(true);
    expect(plan.localActionsOnly).toBe(true);
    expect(plan.observedButUnresolved).toBe(false);
    expect(plan.candidates).toEqual([]);
  });

  it('marks observedButUnresolved when observed but the manifest is unavailable', () => {
    const plan = planReportRouting(indexedProvenance, {});
    expect(plan.provenanceUnknown).toBe(false);
    expect(plan.observedButUnresolved).toBe(true);
    expect(plan.localActionsOnly).toBe(true);
    expect(plan.candidates).toEqual([]);
  });
});

describe('planAppealReportRouting', () => {
  it('routes only to a report endpoint whose node id matches the signal issuer', () => {
    const plan = planAppealReportRouting('issuer-node', {
      'https://issuer.example': manifest({ node_id: 'issuer-node' }),
      'https://other.example': manifest({ node_id: 'other-node' }),
    });

    expect(plan.candidates).toHaveLength(1);
    expect(plan.candidates[0].target.nodeId).toBe('issuer-node');
    expect(plan.candidates[0].target.capability).toBe('trust_signal');
    expect(plan.candidates[0].contact).toEqual({
      kind: 'endpoint',
      value: 'https://node.example/v1/report',
    });
  });

  // #702: 異議申し立ても同じ判定(community_local_trust + trust_signals_issued_by_this_node)を使う。
  it('does not route an appeal to the issuer when it no longer provides trust signals', () => {
    const plan = planAppealReportRouting('issuer-node', {
      'https://issuer.example': manifest({
        node_id: 'issuer-node',
        capability_scope: { available_enabled: ['community_index'], planned_enabled: [] },
        authority_scope: { applies_to: ['this_node', 'communities_indexed_by_this_node'], does_not_apply_to: [] },
      }),
    });
    expect(plan.candidates).toHaveLength(0);
    expect(plan.observedButUnresolved).toBe(true);
  });

  it('does not fall back to contact or a different node when issuer routing is unavailable', () => {
    const plan = planAppealReportRouting('issuer-node', {
      'https://issuer.example': manifest({ node_id: 'issuer-node', report_endpoint: '' }),
      'https://other.example': manifest({ node_id: 'other-node' }),
    });

    expect(plan.candidates).toEqual([]);
    expect(plan.observedButUnresolved).toBe(true);
    expect(plan.localActionsOnly).toBe(true);
  });
});
