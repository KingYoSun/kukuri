import { invokeCommand } from '@/lib/api/tauriClient';

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
}

export const accessControlApi = {
  issueInvite: (request: AccessControlIssueInviteRequest) =>
    invokeCommand<AccessControlIssueInviteResponse>('access_control_issue_invite', { request }),
  requestJoin: (request: AccessControlJoinRequest) =>
    invokeCommand<AccessControlJoinResponse>('access_control_request_join', { request }),
};
