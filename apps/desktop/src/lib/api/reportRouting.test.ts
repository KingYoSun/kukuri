import { describe, expect, it } from 'vitest';

import { type ContentProvenance, unknownProvenance } from './provenance';
import {
  manifestToReportTarget,
  nodeAcceptsReportForCapability,
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
    capability_scope: { available_enabled: ['community_index'], planned_enabled: [] },
    authority_scope: { applies_to: ['this_node'], does_not_apply_to: ['user_identity'] },
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
      authority_scope: { applies_to: ['this_node'], does_not_apply_to: ['moderation'] },
    });
    expect(nodeAcceptsReportForCapability(m, 'moderation')).toBe(false);
    expect(nodeAcceptsReportForCapability(m, 'community_index')).toBe(true);
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
      authorityScope: ['this_node'],
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
