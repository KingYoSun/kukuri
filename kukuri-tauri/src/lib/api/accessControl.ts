import { invokeCommand, invokeCommandVoid } from '@/lib/api/tauriClient';

export interface AccessControlIssueInviteRequest {
  topic_id: string;
  expires_in?: number | null;
  max_uses?: number | null;
  nonce?: string | null;
}

export interface AccessControlIssueInviteResponse {
  invite_event_json: unknown;
}

export interface AccessControlJoinRequest {
  topic_id?: string;
  scope?: string;
  invite_event_json?: unknown;
  target_pubkey?: string;
  broadcast_to_topic?: boolean;
}

export interface AccessControlJoinResponse {
  event_id: string;
  sent_topics: string[];
  event_json: unknown;
}

export interface AccessControlPendingJoinRequest {
  event_id: string;
  topic_id: string;
  scope: string;
  requester_pubkey: string;
  target_pubkey?: string | null;
  requested_at?: number | null;
  received_at: number;
  invite_event_json?: unknown;
}

export interface AccessControlListJoinRequestsResponse {
  items: AccessControlPendingJoinRequest[];
}

export interface AccessControlApproveJoinRequest {
  event_id: string;
}

export interface AccessControlApproveJoinResponse {
  event_id: string;
  key_envelope_event_id: string;
  key_envelope_event_json: unknown;
}

export interface AccessControlRejectJoinRequest {
  event_id: string;
}

export interface AccessControlIngestEventRequest {
  event_json: unknown;
}

export const accessControlApi = {
  issueInvite: (request: AccessControlIssueInviteRequest) =>
    invokeCommand<AccessControlIssueInviteResponse>('access_control_issue_invite', { request }),
  requestJoin: (request: AccessControlJoinRequest) =>
    invokeCommand<AccessControlJoinResponse>('access_control_request_join', { request }),
  listJoinRequests: () =>
    invokeCommand<AccessControlListJoinRequestsResponse>('access_control_list_join_requests'),
  approveJoinRequest: (request: AccessControlApproveJoinRequest) =>
    invokeCommand<AccessControlApproveJoinResponse>('access_control_approve_join_request', {
      request,
    }),
  rejectJoinRequest: (request: AccessControlRejectJoinRequest) =>
    invokeCommandVoid('access_control_reject_join_request', { request }),
  ingestEventJson: (request: AccessControlIngestEventRequest) =>
    invokeCommandVoid('access_control_ingest_event_json', { request }),
};
