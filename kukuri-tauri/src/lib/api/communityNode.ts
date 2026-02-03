import { invokeCommand, invokeCommandVoid } from '@/lib/api/tauriClient';

export type CommunityNodeRoleKey = 'labels' | 'trust' | 'search' | 'bootstrap';

export interface CommunityNodeRoleConfig {
  labels: boolean;
  trust: boolean;
  search: boolean;
  bootstrap: boolean;
}

export const defaultCommunityNodeRoles: CommunityNodeRoleConfig = {
  labels: true,
  trust: true,
  search: false,
  bootstrap: true,
};

export interface CommunityNodeConfigNodeRequest {
  base_url: string;
  roles?: CommunityNodeRoleConfig;
}

export interface CommunityNodeConfigNodeResponse {
  base_url: string;
  roles: CommunityNodeRoleConfig;
  has_token: boolean;
  token_expires_at: number | null;
  pubkey: string | null;
}

export interface CommunityNodeConfigResponse {
  nodes: CommunityNodeConfigNodeResponse[];
}

export interface CommunityNodeAuthResponse {
  expires_at: number;
  pubkey: string;
}

export interface GroupKeyEntry {
  topic_id: string;
  scope: string;
  epoch: number;
  stored_at: number;
}

export interface CommunityNodeLabelsRequest {
  base_url?: string;
  target: string;
  topic?: string;
  limit?: number;
  cursor?: string | null;
}

export interface CommunityNodeTrustRequest {
  base_url?: string;
  subject: string;
}

export interface CommunityNodeReportRequest {
  base_url?: string;
  report_event_json?: unknown;
  target?: string;
  reason?: string;
}

export interface CommunityNodeTrustAnchorRequest {
  attester: string;
  claim?: string;
  topic?: string;
  weight?: number;
}

export interface CommunityNodeTrustAnchorState {
  attester: string;
  claim?: string;
  topic?: string;
  weight: number;
  issued_at: number;
  event_json: unknown;
}

export interface CommunityNodeSearchRequest {
  base_url?: string;
  topic: string;
  q?: string;
  limit?: number;
  cursor?: string | null;
}

export interface CommunityNodeSearchHit {
  event_id?: string;
  topic_id?: string;
  kind?: number;
  author?: string;
  created_at?: number;
  title?: string;
  summary?: string;
  content?: string;
  tags?: string[];
}

export interface CommunityNodeSearchResponse {
  topic: string;
  query?: string | null;
  items: CommunityNodeSearchHit[];
  next_cursor: string | null;
  total: number;
}

export interface CommunityNodeConsentRequest {
  base_url?: string;
  policy_ids?: string[];
  accept_all_current?: boolean;
}

export const communityNodeApi = {
  getConfig: () => invokeCommand<CommunityNodeConfigResponse | null>('get_community_node_config'),

  setConfig: (nodes: CommunityNodeConfigNodeRequest[]) =>
    invokeCommand<CommunityNodeConfigResponse>('set_community_node_config', {
      request: { nodes },
    }),

  clearConfig: () => invokeCommandVoid('clear_community_node_config'),

  authenticate: (baseUrl: string) =>
    invokeCommand<CommunityNodeAuthResponse>('community_node_authenticate', {
      request: { base_url: baseUrl },
    }),

  clearToken: (baseUrl: string) =>
    invokeCommandVoid('community_node_clear_token', {
      request: { base_url: baseUrl },
    }),

  listGroupKeys: () => invokeCommand<GroupKeyEntry[]>('community_node_list_group_keys'),

  listLabels: (request: CommunityNodeLabelsRequest) =>
    invokeCommand<Record<string, unknown>>('community_node_list_labels', { request }),

  submitReport: (request: CommunityNodeReportRequest) =>
    invokeCommand<Record<string, unknown>>('community_node_submit_report', { request }),

  trustReportBased: (request: CommunityNodeTrustRequest) =>
    invokeCommand<Record<string, unknown>>('community_node_trust_report_based', { request }),

  trustCommunicationDensity: (request: CommunityNodeTrustRequest) =>
    invokeCommand<Record<string, unknown>>('community_node_trust_communication_density', {
      request,
    }),

  search: (request: CommunityNodeSearchRequest) =>
    invokeCommand<CommunityNodeSearchResponse>('community_node_search', { request }),

  listBootstrapNodes: () =>
    invokeCommand<Record<string, unknown>>('community_node_list_bootstrap_nodes'),

  listBootstrapServices: (topicId: string) =>
    invokeCommand<Record<string, unknown>>('community_node_list_bootstrap_services', {
      request: { topic_id: topicId },
    }),

  getConsentStatus: (baseUrl: string) =>
    invokeCommand<Record<string, unknown>>('community_node_get_consent_status', {
      request: { base_url: baseUrl },
    }),

  acceptConsents: (request: CommunityNodeConsentRequest) =>
    invokeCommand<Record<string, unknown>>('community_node_accept_consents', { request }),

  getTrustAnchor: () =>
    invokeCommand<CommunityNodeTrustAnchorState | null>('community_node_get_trust_anchor'),

  setTrustAnchor: (request: CommunityNodeTrustAnchorRequest) =>
    invokeCommand<CommunityNodeTrustAnchorState>('community_node_set_trust_anchor', { request }),

  clearTrustAnchor: () => invokeCommandVoid('community_node_clear_trust_anchor'),
};
