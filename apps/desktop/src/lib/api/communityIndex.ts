import type {
  CommunityNodeConfig,
  CommunityNodeManifest,
  CommunityNodeNodeStatus,
} from './types';

export type CommunityIndexManifestEntry =
  | { status: 'absent' | 'loading' }
  | { status: 'error'; error: string }
  | { status: 'ok'; manifest: CommunityNodeManifest };

export function eligibleCommunityIndexNodes(
  config: CommunityNodeConfig,
  statuses: readonly CommunityNodeNodeStatus[],
  manifests: Readonly<Record<string, CommunityIndexManifestEntry>>
): string[] {
  const statusByUrl = new Map(statuses.map((status) => [status.base_url, status]));
  return config.nodes
    .map((node) => node.base_url)
    .filter((baseUrl) => {
      const status = statusByUrl.get(baseUrl);
      const manifest = manifests[baseUrl];
      return Boolean(
        status?.auth_state.authenticated &&
          status.consent_state?.all_required_accepted &&
          !status.last_error &&
          manifest?.status === 'ok' &&
          manifest.manifest.capability_scope.available_enabled.includes('community_index')
      );
    });
}

export function resolveCommunityIndexNodeBaseUrl(
  current: string | null,
  eligibleBaseUrls: readonly string[]
): string | null {
  if (current && eligibleBaseUrls.includes(current)) {
    return current;
  }
  return eligibleBaseUrls[0] ?? null;
}

/// 接続状態の定期更新後などに、現在の構成・接続状態・取得済み構成情報から適格一覧を求め直し、
/// 選択中の索引ノードを再調整する(#698)。構成情報の再取得は行わず、既存の記録だけを使う。
export function reconcileCommunityIndexNodeSelection(state: {
  communityNodeConfig: CommunityNodeConfig;
  communityNodeStatuses: readonly CommunityNodeNodeStatus[];
  communityNodeManifests: Readonly<Record<string, CommunityIndexManifestEntry>>;
  communityIndexNodeBaseUrl: string | null;
}): string | null {
  const eligible = eligibleCommunityIndexNodes(
    state.communityNodeConfig,
    state.communityNodeStatuses,
    state.communityNodeManifests
  );
  return resolveCommunityIndexNodeBaseUrl(state.communityIndexNodeBaseUrl, eligible);
}
