import { startTransition, useCallback, useRef } from 'react';

import type { CommunityNodeNodeStatus, DesktopApi } from '@/lib/api';
import { mergeCommunityNodeStatus } from '@/shell/presentation';
import { useDesktopShellStoreApi, type DesktopShellState, type DesktopShellStateValue } from '@/shell/store';

type Setter<K extends keyof DesktopShellState> = (
  value: DesktopShellStateValue<K>
) => void;

export function useConnectivityStatusRefresh(
  api: DesktopApi,
  setSyncStatus: Setter<'syncStatus'>,
  setCommunityNodeStatuses: Setter<'communityNodeStatuses'>
): () => Promise<CommunityNodeNodeStatus[] | null> {
  const store = useDesktopShellStoreApi();
  const sequence = useRef(0);
  return useCallback(async () => {
    const requestId = ++sequence.current;
    const baseline = store.getState().communityNodeStatuses;
    const [syncStatusResult, communityNodeStatusesResult] = await Promise.allSettled([
      api.getSyncStatus(),
      api.getCommunityNodeStatuses(),
    ]);
    if (requestId !== sequence.current) return null;
    startTransition(() => {
      if (syncStatusResult.status === 'fulfilled') {
        setSyncStatus(syncStatusResult.value);
      }
      if (communityNodeStatusesResult.status === 'fulfilled') {
        setCommunityNodeStatuses((current) => {
          const baselineByUrl = new Map(baseline.map((status) => [status.base_url, status]));
          const currentByUrl = new Map(current.map((status) => [status.base_url, status]));
          const incoming = communityNodeStatusesResult.value;
          const next = incoming.map((status) => {
            const latest = currentByUrl.get(status.base_url);
            // この読込中に受諾/撤回/eventで更新されたNodeを古いsnapshotで巻き戻さない。
            return latest && latest !== baselineByUrl.get(status.base_url)
              ? latest : mergeCommunityNodeStatus(latest, status);
          });
          return [...next, ...current.filter((status) =>
            status !== baselineByUrl.get(status.base_url) &&
            !incoming.some((item) => item.base_url === status.base_url))];
        });
        store.getState().patchState({
          communityNodeStatusesLoaded: true, communityNodeStatusError: null,
        });
      } else if (store.getState().communityNodeStatuses === baseline) {
        store.getState().patchState({ communityNodeStatusError: 'status_unavailable' });
      }
    });
    return communityNodeStatusesResult.status === 'fulfilled'
      ? communityNodeStatusesResult.value
      : null;
  }, [api, setCommunityNodeStatuses, setSyncStatus, store]);
}
