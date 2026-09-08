import type { ReactNode } from 'react';
import { act, renderHook } from '@testing-library/react';
import { expect, test, vi } from 'vitest';
import type { CommunityNodeNodeStatus } from '@/lib/api';
import { createDesktopMockApi } from '@/mocks/desktopApiMock';
import { createDesktopShellStore, DesktopShellStoreContext } from '@/shell/store';
import { useCommunityNodeRecovery } from './useCommunityNodeRecovery';
import { createDeferred } from '@/shell/DesktopShellPage.testHelpers';

async function setup() {
  const api = createDesktopMockApi();
  const store = createDesktopShellStore();
  const config = await api.getCommunityNodeConfig();
  const statuses = await api.getCommunityNodeStatuses();
  store.getState().patchState({ communityNodeConfig: config, communityNodeStatuses: statuses });
  const refresh = vi.fn().mockResolvedValue(statuses);
  const load = vi.fn().mockResolvedValue(undefined);
  const { result } = renderHook(() => useCommunityNodeRecovery(api, refresh, load), {
    wrapper: ({ children }: { children: ReactNode }) => <DesktopShellStoreContext.Provider value={store}>{children}</DesktopShellStoreContext.Provider>,
  });
  const availability = { reason: 'connectionFailed' as const, recovery: 'metadata' as const,
    baseUrl: config.nodes[0].base_url, manual: false, retryAfter: null };
  return { api, store, result, refresh, load, status: statuses[0], availability };
}

test('recovery does not restore an old ready status after consent was withdrawn in flight', async () => {
  const { api, store, result, status, availability } = await setup();
  const deferred = createDeferred<CommunityNodeNodeStatus>();
  vi.spyOn(api, 'refreshCommunityNodeMetadata').mockReturnValue(deferred.promise);
  let recovery!: Promise<void>;
  act(() => { recovery = result.current(availability); });
  const withdrawn = { ...status, auth_state: { authenticated: false, expires_at: null },
    local_consent: { ...status.local_consent!, withdrawn_at: 1234 }, consent_state: null };
  act(() => store.getState().setField('communityNodeStatuses', [withdrawn]));
  await act(async () => { deferred.resolve(status); await recovery; });
  expect(store.getState().communityNodeStatuses[0].local_consent?.withdrawn_at).toBe(1234);
  expect(store.getState().communityNodeStatuses[0].auth_state.authenticated).toBe(false);
});

test.each(['withdrawn', 'updated', 'retry', 'removed'] as const)(
  'recovery checks current %s state before protected metadata work', async (change) => {
    const { api, store, result, status, availability } = await setup();
    const metadata = vi.spyOn(api, 'refreshCommunityNodeMetadata');
    act(() => {
      if (change === 'removed') store.getState().setField('communityNodeConfig', { nodes: [] });
      else store.getState().setField('communityNodeStatuses', [{ ...status,
        ...(change === 'withdrawn' ? { local_consent: { ...status.local_consent!, withdrawn_at: 1234 } } : {}),
        ...(change === 'updated' ? { consent_update_pending: true } : {}),
        ...(change === 'retry' ? { retry_after: Date.now() / 1000 + 100 } : {}),
      }]);
    });
    await act(async () => result.current(availability));
    expect(metadata).not.toHaveBeenCalled();
  }
);
