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
