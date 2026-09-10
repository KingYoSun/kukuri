import { beforeEach, expect, test } from 'vitest';
import { createDesktopMockApi } from '@/mocks/desktopApiMock';
import type { CommunityNodeNodeStatus } from './types';
import { eligibleTesterFeedbackNodes, type CommunityIndexManifestEntry } from './communityIndex';
import { testerFeedbackAvailability } from './testerFeedbackAvailability';

const A = 'https://first.example';
const B = 'https://second.example';
let ready: CommunityNodeNodeStatus;
let pending: CommunityNodeNodeStatus;
let manifests: Record<string, CommunityIndexManifestEntry>;
const config = { nodes: [{ base_url: A }] };

beforeEach(async () => {
  const api = createDesktopMockApi();
  await api.setCommunityNodeConfig(config.nodes);
  pending = (await api.getCommunityNodeStatuses())[0];
  ready = await api.acceptCommunityNodeConsents(A, (await api.fetchCommunityNodePolicies(A)).policies, 'en');
  const manifest = (await api.fetchCommunityNodeManifest(A)).manifest!;
  manifests = { [A]: { status: 'ok', manifest: { ...manifest, node_name: 'Search Only',
    capability_scope: { available_enabled: ['community_index'], planned_enabled: [] },
  } } };
});

function input() {
  return { config, statuses: [ready], manifests, configLoaded: true, statusesLoaded: true,
    configError: false, statusError: false };
}

test('search and consent do not imply feedback intake; returns a named explanation without changing eligibility', () => {
  expect(testerFeedbackAvailability(input())).toEqual({ state: 'unavailable', nodes: [
    { baseUrl: A, label: 'Search Only', reason: 'notProvided' },
  ] });
  expect(eligibleTesterFeedbackNodes(config, [ready], manifests)).toEqual([]);
});

test('distinguishes successful empty config from loading and failed config/status', () => {
  expect(testerFeedbackAvailability({ ...input(), config: { nodes: [] } }).state).toBe('noNodes');
  expect(testerFeedbackAvailability({ ...input(), config: { nodes: [] }, configLoaded: false }).state).toBe('checking');
  expect(testerFeedbackAvailability({ ...input(), config: { nodes: [] }, configError: true }).state).toBe('configUnavailable');
  expect(testerFeedbackAvailability({ ...input(), statusesLoaded: false }).state).toBe('checking');
  expect(testerFeedbackAvailability({ ...input(), statusError: true }).state).toBe('statusUnavailable');
  expect(testerFeedbackAvailability({ ...input(), statuses: [] }).nodes[0].reason).toBe('checking');
});

test('distinguishes initial consent, withdrawn consent, and updates before connection state', () => {
  for (const status of [pending, { ...ready, local_consent: { ...ready.local_consent!, withdrawn_at: 42 } }]) {
    expect(testerFeedbackAvailability({ ...input(), statuses: [status] }).nodes[0].reason).toBe('consentRequired');
  }
  expect(testerFeedbackAvailability({ ...input(), statuses: [{ ...ready, consent_update_pending: true }] }).nodes[0].reason).toBe('reconsentRequired');
  expect(testerFeedbackAvailability({ ...input(), statuses: [{ ...ready,
    consent_state: { ...ready.consent_state!, all_required_accepted: false },
  }] }).nodes[0].reason).toBe('reconsentRequired');
});

test.each(['connecting', 'authenticating', 'accepting', 'refreshing'] as const)('explains %s without treating it as non-support', (session_phase) => {
  expect(testerFeedbackAvailability({ ...input(), statuses: [{ ...ready, session_phase,
    auth_state: pending.auth_state,
  }] }).nodes[0].reason).toBe('connecting');
});

test('reports retry and connection failures without exposing raw diagnostics', () => {
  for (const [session_phase, reason] of [['retrying', 'retrying'], ['ready', 'connectionFailed']] as const) {
    const result = testerFeedbackAvailability({ ...input(), statuses: [{ ...ready, session_phase, last_error: 'secret diagnostic', retry_after: 123 }] });
    expect(result.nodes[0].reason).toBe(reason);
    expect(JSON.stringify(result)).not.toContain('secret diagnostic');
  }
  expect(testerFeedbackAvailability({ ...input(), statuses: [{ ...ready, auth_state: pending.auth_state }] }).nodes[0].reason).toBe('authRequired');
});

test.each(['INVITE_REQUIRED', 'BANNED'] as const)('reports admission restrictions: %s', (code) => {
  expect(testerFeedbackAvailability({ ...input(), statuses: [{ ...ready, session_phase: 'awaiting_admission',
    admission_rejection: { code, message: 'private diagnostic' },
  }] }).nodes[0].reason).toBe('admissionRequired');
});

test.each(['loading', 'absent', 'error'] as const)('does not confuse manifest %s with unsupported intake', (status) => {
  manifests[A] = status === 'error' ? { status, error: 'private diagnostic' } : { status };
  expect(testerFeedbackAvailability(input()).nodes[0]).toMatchObject({ baseUrl: A, label: A,
    reason: status === 'loading' ? 'checking' : 'manifestUnavailable',
  });
});

test('explains each node in a mixed unavailable configuration and ignores removed nodes', () => {
  const result = testerFeedbackAvailability({ ...input(), config: { nodes: [{ base_url: A }, { base_url: B }] },
    statuses: [ready, { ...pending, base_url: B }, { ...ready, base_url: 'https://removed.example' }],
  });
  expect(result.nodes.map(node => [node.baseUrl, node.reason])).toEqual([[A, 'notProvided'], [B, 'consentRequired']]);
});

test('one eligible node remains usable when another is unavailable; updates use current state', () => {
  const entry = manifests[A];
  if (entry.status !== 'ok') throw new Error('fixture');
  const mixed = { ...input(), config: { nodes: [{ base_url: B }, { base_url: A }] } };
  expect(testerFeedbackAvailability(mixed).state).toBe('unavailable');
  entry.manifest.capability_scope.available_enabled.push('tester_feedback');
  expect(testerFeedbackAvailability(mixed)).toEqual({ state: 'ready', nodes: [] });
  expect(eligibleTesterFeedbackNodes(mixed.config, mixed.statuses, manifests)).toEqual([A]);
  expect(testerFeedbackAvailability({ ...mixed, statusError: true }).state).toBe('statusUnavailable');
  expect(testerFeedbackAvailability({ ...mixed, configError: true }).state).toBe('configUnavailable');
  expect(testerFeedbackAvailability({ ...mixed, statuses: [pending] }).state).toBe('unavailable');
});
