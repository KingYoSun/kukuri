import { startTransition, useCallback } from 'react';

import type { CommunityNodeNodeStatus, DesktopApi } from '@/lib/api';
import { mergeCommunityNodeStatuses } from '@/shell/presentation';
import type { DesktopShellState, DesktopShellStateValue } from '@/shell/store';

type Setter<K extends keyof DesktopShellState> = (
  value: DesktopShellStateValue<K>
) => void;

export function useConnectivityStatusRefresh(
  api: DesktopApi,
  setSyncStatus: Setter<'syncStatus'>,
  setCommunityNodeStatuses: Setter<'communityNodeStatuses'>
): () => Promise<CommunityNodeNodeStatus[] | null> {
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
    return communityNodeStatusesResult.status === 'fulfilled'
      ? communityNodeStatusesResult.value
      : null;
  }, [api, setCommunityNodeStatuses, setSyncStatus]);
}
