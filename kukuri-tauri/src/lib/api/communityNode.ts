import { invokeCommand, invokeCommandVoid } from '@/lib/api/tauriClient';

export type CommunityNodeScope = 'public' | 'friend_plus' | 'friend' | 'invite';

export interface CommunityNodeConfigResponse {
  base_url: string;
  has_token: boolean;
  token_expires_at: number | null;
  pubkey: string | null;
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

export interface CommunityNodeKeyEnvelopeResponse {
  stored: GroupKeyEntry[];
}

export interface CommunityNodeRedeemInviteResponse {
  topic_id: string;
  scope: string;
  epoch: number;
}

export interface CommunityNodeLabelsRequest {
  target: string;
  topic?: string;
  limit?: number;
  cursor?: string | null;
}

export interface CommunityNodeTrustRequest {
  subject: string;
}

export interface CommunityNodeSearchRequest {
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
  policy_ids?: string[];
  accept_all_current?: boolean;
}

export interface CommunityNodeKeyEnvelopeRequest {
  topic_id: string;
  scope?: string;
  after_epoch?: number;
}

export const communityNodeApi = {
  getConfig: () => invokeCommand<CommunityNodeConfigResponse | null>('get_community_node_config'),

  setConfig: (baseUrl: string) =>
    invokeCommand<CommunityNodeConfigResponse>('set_community_node_config', {
      request: { base_url: baseUrl },
    }),

  clearConfig: () => invokeCommandVoid('clear_community_node_config'),

  authenticate: () => invokeCommand<CommunityNodeAuthResponse>('community_node_authenticate'),

  clearToken: () => invokeCommandVoid('community_node_clear_token'),

  listGroupKeys: () => invokeCommand<GroupKeyEntry[]>('community_node_list_group_keys'),

  syncKeyEnvelopes: (request: CommunityNodeKeyEnvelopeRequest) =>
    invokeCommand<CommunityNodeKeyEnvelopeResponse>('community_node_sync_key_envelopes', {
      request,
    }),

  redeemInvite: (capabilityEvent: unknown) =>
    invokeCommand<CommunityNodeRedeemInviteResponse>('community_node_redeem_invite', {
      request: { capability_event_json: capabilityEvent },
    }),

  listLabels: (request: CommunityNodeLabelsRequest) =>
    invokeCommand<Record<string, unknown>>('community_node_list_labels', { request }),

  submitReport: (request: Record<string, unknown>) =>
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

  getConsentStatus: () =>
    invokeCommand<Record<string, unknown>>('community_node_get_consent_status'),

  acceptConsents: (request: CommunityNodeConsentRequest) =>
    invokeCommand<Record<string, unknown>>('community_node_accept_consents', { request }),
};
