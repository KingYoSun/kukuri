import { act, renderHook, waitFor } from '@testing-library/react';
import { expect, test, vi } from 'vitest';
import type { CommunityNodeConsentView } from '@/components/settings/types';
import { createCommunityNodePanelFixture } from '@/components/settings/fixtures';
import { useCommunityNodePolicyDialog } from './useCommunityNodePolicyDialog';
import { createDeferred } from '@/shell/DesktopShellPage.testHelpers';

test('Settings/Dome acceptance uses the displayed language and ignores an older catalog after reopen', async () => {
  const nodes = createCommunityNodePanelFixture().nodes;
  const baseUrl = nodes[0].baseUrl;
  const view = (body: string, revision: string): CommunityNodeConsentView => ({
    ...nodes[0].consent, allRequiredAccepted: false, hasLocalConsent: false,
    policies: nodes[0].consent.policies.map((policy) => ({ ...policy, body, policySnapshotRevision: revision, acceptedAtLabel: null })),
  });
  const old = createDeferred<CommunityNodeConsentView>();
  const fetch = vi.fn().mockReturnValueOnce(old.promise).mockResolvedValueOnce(view('English terms', 'en-snapshot'));
  const accept = vi.fn().mockResolvedValue(undefined);
  const { result, rerender } = renderHook(({ language }) => useCommunityNodePolicyDialog({
    nodes, language, fetchPolicies: fetch, acceptPolicies: accept,
  }), { initialProps: { language: 'ja' } });
  act(() => result.current.open(baseUrl));
  await waitFor(() => expect(fetch).toHaveBeenCalledTimes(1));
  act(() => result.current.dialog!.onOpenChange(false));
  rerender({ language: 'en' });
  act(() => result.current.open(baseUrl));
  await waitFor(() => expect(result.current.dialog?.consent.policies[0]?.body).toBe('English terms'));
  await act(async () => old.resolve(view('古い日本語の規約', 'ja-snapshot')));
  expect(result.current.dialog?.consent.policies[0]?.body).toBe('English terms');
  act(() => result.current.dialog!.onAccept());
  await waitFor(() => expect(accept).toHaveBeenCalledWith(baseUrl, [{
    policy_slug: 'terms_of_service', policy_version: 1, policy_snapshot_revision: 'en-snapshot',
  }], 'en'));
});

test('an open policy dialog refreshes on language change before it can accept again', async () => {
  const nodes = createCommunityNodePanelFixture().nodes;
  const firstView = { ...nodes[0].consent, allRequiredAccepted: false };
  const delayed = createDeferred<CommunityNodeConsentView>();
  const fetch = vi.fn().mockResolvedValueOnce(firstView).mockReturnValueOnce(delayed.promise);
  const accept = vi.fn().mockResolvedValue(undefined);
  const { result, rerender } = renderHook(({ language }) => useCommunityNodePolicyDialog({
    nodes, language, fetchPolicies: fetch, acceptPolicies: accept,
  }), { initialProps: { language: 'en' } });
  act(() => result.current.open(nodes[0].baseUrl));
  await waitFor(() => expect(result.current.dialog?.consent.loaded).toBe(true));
  rerender({ language: 'ja' });
  expect(result.current.dialog?.consent.loaded).toBe(false);
  act(() => result.current.dialog!.onAccept());
  expect(accept).not.toHaveBeenCalled();
  await act(async () => delayed.resolve(firstView));
  act(() => result.current.dialog!.onAccept());
  await waitFor(() => expect(accept).toHaveBeenCalledWith(nodes[0].baseUrl, expect.any(Array), 'ja'));
});
