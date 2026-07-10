import { startTransition, useCallback } from 'react';

import type { DesktopApi } from '@/lib/api';
import { mergeCommunityNodeStatuses } from '@/shell/selectors';
import type { DesktopShellState, DesktopShellStateValue } from '@/shell/store';

type Setter<K extends keyof DesktopShellState> = (
  value: DesktopShellStateValue<K>
) => void;

export function useConnectivityStatusRefresh(
  api: DesktopApi,
  setSyncStatus: Setter<'syncStatus'>,
  setCommunityNodeStatuses: Setter<'communityNodeStatuses'>
): () => Promise<void> {
  return useCallback(async () => {
    const [syncStatusResult, communityNodeStatusesResult] = await Promise.allSettled([
      api.getSyncStatus(),
      api.getCommunityNodeStatuses(),
    ]);
    startTransition(() => {
      if (syncStatusResult.status === 'fulfilled') {
        setSyncStatus(syncStatusResult.value);
      }
      if (communityNodeStatusesResult.status === 'fulfilled') {
        setCommunityNodeStatuses((current) =>
          mergeCommunityNodeStatuses(current, communityNodeStatusesResult.value)
        );
      }
    });
  }, [api, setCommunityNodeStatuses, setSyncStatus]);
}
