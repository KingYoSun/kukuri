import { act, renderHook, waitFor } from '@testing-library/react';
import { expect, test, vi } from 'vitest';
import i18n from '@/i18n';
import { createDesktopMockApi } from '@/mocks/desktopApiMock';
import { useCommunityNodeConsentFlow } from './useCommunityNodeConsentFlow';
import { createDeferred } from '@/shell/DesktopShellPage.testHelpers';
import type { CommunityNodePoliciesResponse } from '@/lib/api';

const A = 'https://first.example';
const B = 'https://second.example';

async function setup() {
  const api = createDesktopMockApi();
  await api.setCommunityNodeConfig([{ base_url: A }, { base_url: B }]);
  return api;
}

test('policy failure retries within the same node and closing never accepts', async () => {
  const api = await setup();
  const fetch = vi.spyOn(api, 'fetchCommunityNodePolicies').mockRejectedValueOnce(new Error('offline'));
  const accept = vi.spyOn(api, 'acceptCommunityNodeConsents');
  const { result } = renderHook(() => useCommunityNodeConsentFlow({ api, configuredBaseUrls: [A, B] }));
  act(() => result.current.open(A));
  await waitFor(() => expect(result.current.dialog?.consent.loadError).toBe('offline'));
  act(() => result.current.dialog!.onRetry());
  await waitFor(() => expect(result.current.dialog?.consent.loaded).toBe(true));
  expect(fetch).toHaveBeenNthCalledWith(2, A, 'en');
  act(() => result.current.close());
  expect(result.current.dialog).toBeNull();
  expect(accept).not.toHaveBeenCalled();
});

test('late policy response cannot replace the newly selected node or language', async () => {
  const api = await setup();
  const original = await api.fetchCommunityNodePolicies(A);
  const old = createDeferred<CommunityNodePoliciesResponse>();
  vi.spyOn(api, 'fetchCommunityNodePolicies').mockReturnValueOnce(old.promise).mockImplementation(async (baseUrl, language) => ({
    policies: original.policies.map((policy) => ({ ...policy, body_markdown: `${baseUrl} ${language}` })),
  }));
  const { result } = renderHook(() => useCommunityNodeConsentFlow({ api, configuredBaseUrls: [A, B] }));
  act(() => result.current.open(A));
  act(() => result.current.open(B));
  await waitFor(() => expect(result.current.dialog?.consent.policies[0]?.body).toBe(`${B} en`));
  await act(async () => { await i18n.changeLanguage('ja'); });
  await waitFor(() => expect(result.current.dialog?.consent.policies[0]?.body).toBe(`${B} ja`));
  await act(async () => old.resolve(original));
  expect(result.current.dialog?.baseUrl).toBe(B);
  expect(result.current.dialog?.consent.policies[0]?.body).toBe(`${B} ja`);
});

test('submits the displayed snapshot once and keeps a failed acceptance in the dialog', async () => {
  const api = await setup();
  const original = await api.fetchCommunityNodePolicies(A);
  vi.spyOn(api, 'fetchCommunityNodePolicies').mockResolvedValue({ policies: original.policies.map((policy) => ({
    ...policy, policy_snapshot_revision: 'shown-revision',
  })) });
  const save = createDeferred<void>();
  const accept = vi.fn().mockReturnValueOnce(save.promise).mockResolvedValue(undefined);
  const { result } = renderHook(() => useCommunityNodeConsentFlow({ api, configuredBaseUrls: [A, B], acceptConsents: accept }));
  act(() => result.current.open(A));
  await waitFor(() => expect(result.current.dialog?.consent.loaded).toBe(true));
  act(() => { result.current.dialog!.onAccept(); result.current.dialog!.onAccept(); });
  await waitFor(() => expect(accept).toHaveBeenCalledTimes(1));
  expect(accept).toHaveBeenCalledWith(A, original.policies.map((policy) => ({
    policy_slug: policy.policy_slug, policy_version: policy.policy_version, policy_snapshot_revision: 'shown-revision',
  })), 'en');
  await act(async () => save.reject(new Error('storage unavailable')));
  expect(result.current.dialog?.error).toContain('Consent could not be completed');
  act(() => result.current.dialog!.onAccept());
  await waitFor(() => expect(result.current.dialog).toBeNull());
  expect(accept).toHaveBeenCalledTimes(2);
});

test('a node removed while its policies load cannot be accepted', async () => {
  const api = await setup();
  const original = await api.fetchCommunityNodePolicies(A);
  const deferred = createDeferred<CommunityNodePoliciesResponse>();
  vi.spyOn(api, 'fetchCommunityNodePolicies').mockReturnValue(deferred.promise);
  const accept = vi.spyOn(api, 'acceptCommunityNodeConsents');
  const { result, rerender } = renderHook(({ nodes }) => useCommunityNodeConsentFlow({ api, configuredBaseUrls: nodes }), {
    initialProps: { nodes: [A, B] },
  });
  act(() => result.current.open(A));
  rerender({ nodes: [B] });
  await act(async () => deferred.resolve(original));
  expect(result.current.dialog).toBeNull();
  expect(accept).not.toHaveBeenCalled();
});

test('a configuration change before submit blocks acceptance even before shell refresh', async () => {
  const api = await setup();
  const accept = vi.spyOn(api, 'acceptCommunityNodeConsents');
  const { result } = renderHook(() => useCommunityNodeConsentFlow({ api, configuredBaseUrls: [A, B] }));
  act(() => result.current.open(A));
  await waitFor(() => expect(result.current.dialog?.consent.loaded).toBe(true));
  await api.setCommunityNodeConfig([{ base_url: B }]);
  await act(async () => result.current.dialog!.onAccept());
  expect(accept).not.toHaveBeenCalled();
});
