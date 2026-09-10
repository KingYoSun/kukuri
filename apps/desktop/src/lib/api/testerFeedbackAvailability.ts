import type { CommunityNodeConfig, CommunityNodeNodeStatus } from './types';
import {
  communityIndexNodeLabel,
  eligibleTesterFeedbackNodes,
  type CommunityIndexManifestEntry,
} from './communityIndex';

export type TesterFeedbackNodeReason =
  | 'checking' | 'consentRequired' | 'reconsentRequired' | 'authRequired'
  | 'connecting' | 'retrying' | 'connectionFailed' | 'admissionRequired'
  | 'manifestUnavailable' | 'notProvided';

export type TesterFeedbackAvailability = {
  state: 'ready' | 'checking' | 'configUnavailable' | 'statusUnavailable' | 'noNodes' | 'unavailable';
  nodes: Array<{ baseUrl: string; label: string; reason: TesterFeedbackNodeReason }>;
};

// 説明専用のprojection。送信許可は既存eligibleTesterFeedbackNodesが所有する。
export function testerFeedbackAvailability({
  config, statuses, manifests, configLoaded, statusesLoaded, configError, statusError,
}: {
  config: CommunityNodeConfig;
  statuses: readonly CommunityNodeNodeStatus[];
  manifests: Readonly<Record<string, CommunityIndexManifestEntry>>;
  configLoaded: boolean;
  statusesLoaded: boolean;
  configError: boolean;
  statusError: boolean;
}): TesterFeedbackAvailability {
  if (configError) return { state: 'configUnavailable', nodes: [] };
  if (!configLoaded) return { state: 'checking', nodes: [] };
  if (config.nodes.length === 0) return { state: 'noNodes', nodes: [] };
  if (statusError) return { state: 'statusUnavailable', nodes: [] };
  if (!statusesLoaded) return { state: 'checking', nodes: [] };
  if (eligibleTesterFeedbackNodes(config, statuses, manifests).length > 0) {
    return { state: 'ready', nodes: [] };
  }

  const byUrl = new Map(statuses.map(status => [status.base_url, status]));
  return { state: 'unavailable', nodes: config.nodes.map(({ base_url: baseUrl }) => ({
    baseUrl,
    label: communityIndexNodeLabel(baseUrl, manifests[baseUrl]),
    reason: nodeReason(byUrl.get(baseUrl), manifests[baseUrl]),
  })) };
}

function nodeReason(
  status: CommunityNodeNodeStatus | undefined,
  manifest: CommunityIndexManifestEntry | undefined,
): TesterFeedbackNodeReason {
  if (!status) return 'checking';
  if (status.consent_update_pending) return 'reconsentRequired';
  if (status.local_consent && (!status.local_consent.records.length || status.local_consent.withdrawn_at != null)) {
    return 'consentRequired';
  }
  if (status.admission_rejection || status.session_phase === 'awaiting_admission') return 'admissionRequired';
  if (status.session_phase === 'retrying') return 'retrying';
  if (['connecting', 'authenticating', 'accepting', 'refreshing'].includes(status.session_phase ?? '')) return 'connecting';
  if (status.last_error) return 'connectionFailed';
  if (!status.auth_state.authenticated) return 'authRequired';
  if (!status.consent_state?.all_required_accepted) return 'reconsentRequired';
  if (!manifest || manifest.status === 'loading') return 'checking';
  if (manifest.status !== 'ok') return 'manifestUnavailable';
  return 'notProvided';
}
