import createClient from 'openapi-fetch';

import type { paths } from '../generated/admin-api';
import type { DashboardSnapshot } from './types';

const baseUrl = import.meta.env.VITE_ADMIN_API_BASE ?? '/admin-api';
const joinedBaseUrl = baseUrl.replace(/\/$/, '');

type ApiError = Error & { status?: number; payload?: unknown };

const client = createClient<paths>({
  baseUrl,
  credentials: 'include',
  headers: {
    'Content-Type': 'application/json'
  }
});

const toError = (response: Response, payload: unknown): ApiError => {
  const message =
    typeof payload === 'object' && payload !== null
      ? (payload as { message?: string; code?: string }).message ??
        (payload as { message?: string; code?: string }).code ??
        'Request failed'
      : 'Request failed';
  const error = new Error(message) as ApiError;
  error.status = response.status;
  error.payload = payload;
  return error;
};

const unwrap = async <T>(
  request: Promise<{ data?: T; error?: unknown; response: Response }>
): Promise<T> => {
  const { data, error, response } = await request;
  if (response.status === 204) {
    return null as T;
  }
  if (typeof error !== 'undefined') {
    throw toError(response, error);
  }
  return data as T;
};

const buildApiUrl = (path: string) =>
  `${joinedBaseUrl}${path.startsWith('/') ? path : `/${path}`}`;

const parseFetchPayload = async (response: Response): Promise<unknown> => {
  const raw = await response.text();
  if (raw.trim() === '') {
    return null;
  }
  try {
    return JSON.parse(raw) as unknown;
  } catch {
    return raw;
  }
};

export const api = {
  login: (username: string, password: string) =>
    unwrap(client.POST('/v1/admin/auth/login', { body: { username, password } })),
  logout: () => unwrap(client.POST('/v1/admin/auth/logout')),
  me: () => unwrap(client.GET('/v1/admin/auth/me')),
  dashboard: async () => {
    const response = await fetch(buildApiUrl('/v1/admin/dashboard'), {
      method: 'GET',
      credentials: 'include',
      headers: { 'Content-Type': 'application/json' }
    });
    const payload = await parseFetchPayload(response);
    if (!response.ok) {
      throw toError(response, payload);
    }
    return payload as DashboardSnapshot;
  },
  services: () => unwrap(client.GET('/v1/admin/services')),
  updateServiceConfig: (service: string, configJson: unknown, expectedVersion?: number) => {
    const body: { config_json: unknown; expected_version?: number } = { config_json: configJson };
    if (typeof expectedVersion === 'number') {
      body.expected_version = expectedVersion;
    }
    return unwrap(
      client.PUT('/v1/admin/services/{service}/config', {
        params: { path: { service } },
        body
      })
    );
  },
  subscriptionRequests: (status?: string) =>
    unwrap(
      client.GET('/v1/admin/subscription-requests', {
        params: { query: { status } }
      })
    ),
  approveRequest: (requestId: string, reviewNote?: string) =>
    unwrap(
      client.POST('/v1/admin/subscription-requests/{request_id}/approve', {
        params: { path: { request_id: requestId } },
        body: { review_note: reviewNote ?? null }
      })
    ),
  rejectRequest: (requestId: string, reviewNote?: string) =>
    unwrap(
      client.POST('/v1/admin/subscription-requests/{request_id}/reject', {
        params: { path: { request_id: requestId } },
        body: { review_note: reviewNote ?? null }
      })
    ),
  nodeSubscriptions: () => unwrap(client.GET('/v1/admin/node-subscriptions')),
  updateNodeSubscription: (topicId: string, enabled: boolean) =>
    unwrap(
      client.PUT('/v1/admin/node-subscriptions/{topic_id}', {
        params: { path: { topic_id: topicId } },
        body: { enabled }
      })
    ),
  policies: () => unwrap(client.GET('/v1/admin/policies')),
  createPolicy: (payload: {
    policy_type: string;
    version: string;
    locale: string;
    title: string;
    content_md: string;
  }) => unwrap(client.POST('/v1/admin/policies', { body: payload })),
  updatePolicy: (policyId: string, payload: { title: string; content_md: string }) =>
    unwrap(
      client.PUT('/v1/admin/policies/{policy_id}', {
        params: { path: { policy_id: policyId } },
        body: payload
      })
    ),
  publishPolicy: (policyId: string, effectiveAt?: number) =>
    unwrap(
      client.POST('/v1/admin/policies/{policy_id}/publish', {
        params: { path: { policy_id: policyId } },
        body: typeof effectiveAt === 'number' ? { effective_at: effectiveAt } : {}
      })
    ),
  makeCurrentPolicy: (policyId: string) =>
    unwrap(
      client.POST('/v1/admin/policies/{policy_id}/make-current', {
        params: { path: { policy_id: policyId } }
      })
    ),
  auditLogs: (query?: { action?: string; target?: string; since?: number; limit?: number }) =>
    unwrap(
      client.GET('/v1/admin/audit-logs', {
        params: {
          query: {
            action: query?.action,
            target: query?.target,
            since: query?.since,
            limit: query?.limit
          }
        }
      })
    ),
  plans: () => unwrap(client.GET('/v1/admin/plans')),
  createPlan: (payload: {
    plan_id: string;
    name: string;
    is_active: boolean;
    limits: Array<{ metric: string; window: string; limit: number }>;
  }) => unwrap(client.POST('/v1/admin/plans', { body: payload })),
  updatePlan: (
    planId: string,
    payload: {
      name: string;
      is_active: boolean;
      limits: Array<{ metric: string; window: string; limit: number }>;
    }
  ) =>
    unwrap(
      client.PUT('/v1/admin/plans/{plan_id}', {
        params: { path: { plan_id: planId } },
        body: { ...payload, plan_id: planId }
      })
    ),
  subscriptions: (pubkey?: string) =>
    unwrap(
      client.GET('/v1/admin/subscriptions', {
        params: { query: { pubkey } }
      })
    ),
  updateSubscription: (pubkey: string, payload: { plan_id: string; status: string }) =>
    unwrap(
      client.PUT('/v1/admin/subscriptions/{subscriber_pubkey}', {
        params: { path: { subscriber_pubkey: pubkey } },
        body: payload
      })
    ),
  usage: (pubkey: string, metric?: string, days?: number) =>
    unwrap(
      client.GET('/v1/admin/usage', {
        params: { query: { pubkey, metric, days } }
      })
    ),
  moderationRules: (enabled?: boolean) =>
    unwrap(
      client.GET('/v1/admin/moderation/rules', {
        params: { query: { enabled } }
      })
    ),
  createModerationRule: (payload: {
    name: string;
    description?: string | null;
    is_enabled?: boolean;
    priority?: number;
    conditions: unknown;
    action: unknown;
  }) => unwrap(client.POST('/v1/admin/moderation/rules', { body: payload })),
  updateModerationRule: (
    ruleId: string,
    payload: {
      name: string;
      description?: string | null;
      is_enabled?: boolean;
      priority?: number;
      conditions: unknown;
      action: unknown;
    }
  ) =>
    unwrap(
      client.PUT('/v1/admin/moderation/rules/{rule_id}', {
        params: { path: { rule_id: ruleId } },
        body: payload
      })
    ),
  deleteModerationRule: (ruleId: string) =>
    unwrap(
      client.DELETE('/v1/admin/moderation/rules/{rule_id}', {
        params: { path: { rule_id: ruleId } }
      })
    ),
  moderationReports: (query?: {
    target?: string;
    reporter_pubkey?: string;
    since?: number;
    limit?: number;
  }) =>
    unwrap(
      client.GET('/v1/admin/moderation/reports', {
        params: {
          query: {
            target: query?.target,
            reporter_pubkey: query?.reporter_pubkey,
            since: query?.since,
            limit: query?.limit
          }
        }
      })
    ),
  moderationLabels: (query?: { target?: string; topic?: string; since?: number; limit?: number }) =>
    unwrap(
      client.GET('/v1/admin/moderation/labels', {
        params: {
          query: {
            target: query?.target,
            topic: query?.topic,
            since: query?.since,
            limit: query?.limit
          }
        }
      })
    ),
  trustJobs: (query?: {
    status?: string;
    job_type?: string;
    subject_pubkey?: string;
    limit?: number;
  }) =>
    unwrap(
      client.GET('/v1/admin/trust/jobs', {
        params: {
          query: {
            status: query?.status,
            job_type: query?.job_type,
            subject_pubkey: query?.subject_pubkey,
            limit: query?.limit
          }
        }
      })
    ),
  createTrustJob: (payload: { job_type: string; subject_pubkey?: string | null }) =>
    unwrap(client.POST('/v1/admin/trust/jobs', { body: payload })),
  trustSchedules: () => unwrap(client.GET('/v1/admin/trust/schedules')),
  updateTrustSchedule: (jobType: string, payload: { interval_seconds: number; is_enabled: boolean }) =>
    unwrap(
      client.PUT('/v1/admin/trust/schedules/{job_type}', {
        params: { path: { job_type: jobType } },
        body: payload
      })
    ),
  rotateAccessControl: (payload: { topic_id: string; scope: string }) =>
    unwrap(client.POST('/v1/admin/access-control/rotate', { body: payload })),
  accessControlMemberships: (query?: {
    topic_id?: string;
    scope?: string;
    pubkey?: string;
    status?: string;
    limit?: number;
  }) =>
    unwrap(
      client.GET('/v1/admin/access-control/memberships', {
        params: {
          query: {
            topic_id: query?.topic_id,
            scope: query?.scope,
            pubkey: query?.pubkey,
            status: query?.status,
            limit: query?.limit
          }
        }
      })
    ),
  revokeAccessControl: (payload: {
    topic_id: string;
    scope: string;
    pubkey: string;
    reason?: string | null;
  }) => unwrap(client.POST('/v1/admin/access-control/revoke', { body: payload })),
  reindex: (topicId?: string | null) => {
    const body: { topic_id?: string | null } = {};
    if (typeof topicId === 'string' && topicId.trim() !== '') {
      body.topic_id = topicId.trim();
    } else if (topicId === null) {
      body.topic_id = null;
    }
    return unwrap(client.POST('/v1/reindex', { body }));
  },
  createManualLabel: (payload: {
    target: string;
    label: string;
    confidence?: number | null;
    exp: number;
    policy_url: string;
    policy_ref: string;
    topic_id?: string | null;
  }) => unwrap(client.POST('/v1/admin/moderation/labels', { body: payload }))
};
