import { useCallback } from 'react';
import type { CommunityNodeNodeStatus, DesktopApi } from '@/lib/api';
import type { CommunityNodeAvailability } from '@/lib/api/communityNodeAvailability';
import { hasActiveCommunityNodeConsent } from '@/lib/api/communityNodeAvailability';
import { syncCommunityNodeConfigWithStatus, upsertCommunityNodeStatus } from '@/shell/presentation';
import { useDesktopShellStoreApi } from '@/shell/store';

export function useCommunityNodeRecovery(
  api: DesktopApi,
  refreshStatus: () => Promise<CommunityNodeNodeStatus[] | null>,
  loadCapability: () => Promise<void>
) {
  const store = useDesktopShellStoreApi();
  return useCallback(async ({ baseUrl, recovery }: CommunityNodeAvailability) => {
    const state = store.getState();
    const status = state.communityNodeStatuses.find((node) => node.base_url === baseUrl);
    if (baseUrl && !state.communityNodeConfig.nodes.some((node) => node.base_url === baseUrl)) return;
    if (recovery === 'metadata' && baseUrl) {
      // 表示後に撤回/再同意/retryへ遷移していても、古いbuttonで境界を迂回しない。
      if (!status || !hasActiveCommunityNodeConsent(status) || status.consent_update_pending ||
        (status.retry_after ?? 0) > Date.now() / 1000) return;
      const refreshed = await api.refreshCommunityNodeMetadata(baseUrl);
      const current = store.getState();
      if (!current.communityNodeConfig.nodes.some((node) => node.base_url === baseUrl)) return;
      if (current.communityNodeStatuses.find((node) => node.base_url === baseUrl) === status) {
        current.patchState({
          communityNodeStatuses: upsertCommunityNodeStatus(current.communityNodeStatuses, refreshed),
          communityNodeConfig: syncCommunityNodeConfigWithStatus(current.communityNodeConfig, refreshed),
        });
      }
    }
    await refreshStatus();
    await loadCapability();
    if (store.getState().communityNodeStatusError || store.getState().communityNodeConfigError) {
      throw new Error('community node status unavailable');
    }
  }, [api, loadCapability, refreshStatus, store]);
}
