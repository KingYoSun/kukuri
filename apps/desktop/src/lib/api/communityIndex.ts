import type {
  CommunityNodeConfig,
  CommunityNodeManifest,
  CommunityNodeNodeStatus,
} from './types';

export type CommunityIndexManifestEntry =
  | { status: 'absent' | 'loading' }
  | { status: 'error'; error: string }
  | { status: 'ok'; manifest: CommunityNodeManifest };

export type CommunityIndexNodePreference =
  | { mode: 'auto' }
  | { mode: 'manual'; baseUrl: string };

export type CommunityIndexNodeResolution = {
  preference: CommunityIndexNodePreference;
  selectedBaseUrl: string | null;
};

/// 索引・信頼関係・距離利用停止で共通の利用可否境界(#663 / #698 / #705)。
/// 認証済み・必須同意承認済み・通信エラーなし・公開ノード情報が取得済みで、`capabilities` の
/// いずれかが提供中(`available_enabled`)のノードだけを返す。
export function eligibleCommunityNodes(
  config: CommunityNodeConfig,
  statuses: readonly CommunityNodeNodeStatus[],
  manifests: Readonly<Record<string, CommunityIndexManifestEntry>>,
  capabilities: readonly string[]
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
          capabilities.some((capability) =>
            manifest.manifest.capability_scope.available_enabled.includes(capability)
          )
      );
    });
}

export function eligibleCommunityIndexNodes(
  config: CommunityNodeConfig,
  statuses: readonly CommunityNodeNodeStatus[],
  manifests: Readonly<Record<string, CommunityIndexManifestEntry>>
): string[] {
  return eligibleCommunityNodes(config, statuses, manifests, ['community_index']);
}

/// 信頼評価・関係分析の参照は `community_local_trust` を提供中のノードに限る(#705)。
export function eligibleTrustRelationNodes(
  config: CommunityNodeConfig,
  statuses: readonly CommunityNodeNodeStatus[],
  manifests: Readonly<Record<string, CommunityIndexManifestEntry>>
): string[] {
  return eligibleCommunityNodes(config, statuses, manifests, ['community_local_trust']);
}

/// 距離利用停止は、サーバが索引か信頼のいずれかを提供していれば構成される(#705)。
export function eligibleDistanceOptoutNodes(
  config: CommunityNodeConfig,
  statuses: readonly CommunityNodeNodeStatus[],
  manifests: Readonly<Record<string, CommunityIndexManifestEntry>>
): string[] {
  return eligibleCommunityNodes(config, statuses, manifests, [
    'community_local_trust',
    'community_index',
  ]);
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

export function resolveCommunityIndexNodePreference(
  preference: CommunityIndexNodePreference,
  configuredBaseUrls: readonly string[],
  eligibleBaseUrls: readonly string[]
): CommunityIndexNodeResolution {
  if (preference.mode === 'manual') {
    if (!configuredBaseUrls.includes(preference.baseUrl)) {
      return {
        preference: { mode: 'auto' },
        selectedBaseUrl: eligibleBaseUrls[0] ?? null,
      };
    }
    return {
      preference,
      selectedBaseUrl: eligibleBaseUrls.includes(preference.baseUrl)
        ? preference.baseUrl
        : null,
    };
  }
  return { preference, selectedBaseUrl: eligibleBaseUrls[0] ?? null };
}

export function reconcileCommunityIndexNodePreference(state: {
  communityNodeConfig: CommunityNodeConfig;
  communityNodeStatuses: readonly CommunityNodeNodeStatus[];
  communityNodeManifests: Readonly<Record<string, CommunityIndexManifestEntry>>;
  communityIndexNodePreference: CommunityIndexNodePreference;
}): CommunityIndexNodeResolution {
  const configured = state.communityNodeConfig.nodes.map((node) => node.base_url);
  const eligible = eligibleCommunityIndexNodes(
    state.communityNodeConfig,
    state.communityNodeStatuses,
    state.communityNodeManifests
  );
  return resolveCommunityIndexNodePreference(
    state.communityIndexNodePreference,
    configured,
    eligible
  );
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
