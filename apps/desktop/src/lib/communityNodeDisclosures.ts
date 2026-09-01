import type { CommunityNodeConfig } from '@/lib/api/types.generated';
import type { CommunityNodeManifestEntry } from '@/shell/slices/connectivity';

export type CommunityNodeDisclosureLink = {
  key: 'terms' | 'privacy' | 'externalTransmission' | 'abusePolicy' | 'dataRetention';
  href: string;
};

export type CommunityNodeDisclosure = {
  baseUrl: string;
  nodeName: string | null;
  links: CommunityNodeDisclosureLink[];
  manifestAvailable: boolean;
};

export function buildCommunityNodeDisclosures(
  config: CommunityNodeConfig,
  manifests: Record<string, CommunityNodeManifestEntry>
): CommunityNodeDisclosure[] {
  return config.nodes.map((node) => {
    const entry = manifests[node.base_url];
    if (entry?.status !== 'ok') {
      return {
        baseUrl: node.base_url,
        nodeName: null,
        links: [],
        manifestAvailable: false,
      };
    }

    const manifest = entry.manifest;
    const candidates: CommunityNodeDisclosureLink[] = [
      { key: 'terms', href: manifest.terms_url },
      { key: 'privacy', href: manifest.privacy_url },
      { key: 'externalTransmission', href: manifest.external_transmission_url ?? '' },
      { key: 'abusePolicy', href: manifest.abuse_policy_url ?? '' },
      { key: 'dataRetention', href: manifest.data_retention_url ?? '' },
    ];
    return {
      baseUrl: node.base_url,
      nodeName: manifest.node_name.trim() || null,
      links: candidates.filter((link) => link.href.trim().length > 0),
      manifestAvailable: true,
    };
  });
}
