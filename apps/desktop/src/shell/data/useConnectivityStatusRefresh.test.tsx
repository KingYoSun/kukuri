import type { ReactNode } from 'react';
import { act, renderHook } from '@testing-library/react';
import { expect, test, vi } from 'vitest';
import type { CommunityNodeNodeStatus } from '@/lib/api';
import { createDesktopMockApi } from '@/mocks/desktopApiMock';
import { createDesktopShellStore, DesktopShellStoreContext } from '@/shell/store';
import { createDeferred } from '@/shell/DesktopShellPage.testHelpers';
import { useConnectivityStatusRefresh } from './useConnectivityStatusRefresh';

test('an in-flight poll preserves a newer per-node event and refreshes untouched nodes', async () => {
  const api = createDesktopMockApi();
  await api.setCommunityNodeConfig([{ base_url: 'https://a.example' }, { base_url: 'https://b.example' }]);
  const pending = await api.getCommunityNodeStatuses();
  const store = createDesktopShellStore();
  store.getState().setField('communityNodeStatuses', pending);
  const response = createDeferred<CommunityNodeNodeStatus[]>();
  vi.spyOn(api, 'getCommunityNodeStatuses').mockReturnValue(response.promise);
  const { result } = renderHook(() => useConnectivityStatusRefresh(api, vi.fn(), (next) => store.getState().setField('communityNodeStatuses', next)), {
    wrapper: ({ children }: { children: ReactNode }) => <DesktopShellStoreContext.Provider value={store}>{children}</DesktopShellStoreContext.Provider>,
  });
  let refresh!: ReturnType<typeof result.current>;
  act(() => { refresh = result.current(); });
  const accepted = await api.acceptCommunityNodeConsents(pending[0].base_url, (await api.fetchCommunityNodePolicies(pending[0].base_url)).policies, 'en');
  act(() => store.getState().setField('communityNodeStatuses', [accepted, pending[1]]));
  await act(async () => { response.resolve([pending[0], { ...pending[1], last_error: 'new connection failure' }]); await refresh; });
  expect(store.getState().communityNodeStatuses[0].local_consent?.records.length).toBeGreaterThan(0);
  expect(store.getState().communityNodeStatuses[1].last_error).toBe('new connection failure');
});

test('failed status reads are distinguishable from a successfully empty list', async () => {
  const api = createDesktopMockApi();
  const store = createDesktopShellStore();
  vi.spyOn(api, 'getCommunityNodeStatuses').mockRejectedValueOnce(new Error('local status unavailable')).mockResolvedValue([]);
  const { result } = renderHook(() => useConnectivityStatusRefresh(api, vi.fn(), (next) => store.getState().setField('communityNodeStatuses', next)), {
    wrapper: ({ children }: { children: ReactNode }) => <DesktopShellStoreContext.Provider value={store}>{children}</DesktopShellStoreContext.Provider>,
  });
  await act(async () => result.current());
  expect(store.getState().communityNodeStatusesLoaded).toBe(false);
  expect(store.getState().communityNodeStatusError).not.toBeNull();
  await act(async () => result.current());
  expect(store.getState().communityNodeStatusesLoaded).toBe(true);
  expect(store.getState().communityNodeStatusError).toBeNull();
});
