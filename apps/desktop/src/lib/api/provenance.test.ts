import { describe, expect, it } from 'vitest';

import {
  type ContentProvenance,
  type ReportTarget,
  hasResolvableReportTarget,
  isProvenanceUnknown,
  reportTargetContact,
  resolveReportTargets,
  summarizeProvenance,
  unknownProvenance,
} from './provenance';

const indexTarget: ReportTarget = {
  nodeBaseUrl: 'https://node.example',
  capability: 'community_index',
  reportEndpoint: 'https://node.example/v1/report',
  abuseContact: 'abuse@node.example',
};

const knownProvenance: ContentProvenance = {
  canonicalSource: 'author_docs',
  observedVia: [
    {
      nodeBaseUrl: 'https://node.example',
      capability: 'community_index',
      nodeRole: 'community-node',
      manifestVersion: 'v1',
    },
  ],
  responsibleReportTargets: [indexTarget],
};

describe('content provenance', () => {
  it('separates canonical source from observed-via nodes', () => {
    expect(knownProvenance.canonicalSource).toBe('author_docs');
    expect(knownProvenance.observedVia[0].capability).toBe('community_index');
    // 観測経路があっても canonicalSource は author_docs のまま（truth source ではない）。
    expect(knownProvenance.canonicalSource).not.toBe('community_docs');
  });

  it('represents unknown provenance', () => {
    const unknown = unknownProvenance();
    expect(unknown.canonicalSource).toBe('unknown');
    expect(unknown.observedVia).toEqual([]);
    expect(unknown.responsibleReportTargets).toEqual([]);
    expect(isProvenanceUnknown(unknown)).toBe(true);
    expect(isProvenanceUnknown(null)).toBe(true);
    expect(isProvenanceUnknown(undefined)).toBe(true);
    expect(isProvenanceUnknown(knownProvenance)).toBe(false);
  });

  it('resolves responsible report targets', () => {
    expect(resolveReportTargets(knownProvenance)).toEqual([indexTarget]);
    expect(hasResolvableReportTarget(knownProvenance)).toBe(true);
  });

  it('never falls back to a default node when provenance is unknown', () => {
    // unknown / null / undefined はいずれも空配列。default node を合成しない。
    expect(resolveReportTargets(unknownProvenance())).toEqual([]);
    expect(resolveReportTargets(null)).toEqual([]);
    expect(resolveReportTargets(undefined)).toEqual([]);
    expect(hasResolvableReportTarget(unknownProvenance())).toBe(false);
  });

  it('does not synthesize a target when observedVia exists but no report target is provided', () => {
    const observedButNoTarget: ContentProvenance = {
      canonicalSource: 'community_docs',
      observedVia: [{ nodeBaseUrl: 'https://node.example', capability: 'community_index' }],
      responsibleReportTargets: [],
    };
    // 観測経路はあるが通報先未解決 → 空配列（default へ向けない）。
    expect(resolveReportTargets(observedButNoTarget)).toEqual([]);
    expect(hasResolvableReportTarget(observedButNoTarget)).toBe(false);
    // ただし observedVia があるため unknown ではない。
    expect(isProvenanceUnknown(observedButNoTarget)).toBe(false);
  });

  it('prefers report endpoint then abuse contact for contact resolution', () => {
    expect(reportTargetContact(indexTarget)).toEqual({
      kind: 'endpoint',
      value: 'https://node.example/v1/report',
    });
    expect(
      reportTargetContact({
        nodeBaseUrl: 'https://n',
        capability: 'moderation',
        abuseContact: 'abuse@n',
      }),
    ).toEqual({ kind: 'contact', value: 'abuse@n' });
    expect(
      reportTargetContact({ nodeBaseUrl: 'https://n', capability: 'moderation' }),
    ).toEqual({ kind: 'none' });
  });

  it('summarizes provenance for diagnostics', () => {
    const summary = summarizeProvenance(knownProvenance);
    expect(summary.unknown).toBe(false);
    expect(summary.canonicalSource).toBe('author_docs');
    expect(summary.observedVia).toHaveLength(1);
    expect(summary.canReport).toBe(true);

    const unknownSummary = summarizeProvenance(undefined);
    expect(unknownSummary.unknown).toBe(true);
    expect(unknownSummary.canReport).toBe(false);
    expect(unknownSummary.reportTargets).toEqual([]);
  });
});
