const baseUrl = import.meta.env.VITE_ADMIN_API_BASE ?? '/admin-api';

type ApiError = Error & { status?: number; payload?: unknown };

const fetchJson = async (path: string, options: RequestInit = {}) => {
  const response = await fetch(`${baseUrl}${path}`, {
    credentials: 'include',
    headers: {
      'Content-Type': 'application/json',
      ...(options.headers ?? {})
    },
    ...options
  });

  if (response.status === 204) {
    return null;
  }

  const text = await response.text();
  const data = text ? JSON.parse(text) : null;

  if (!response.ok) {
    const message = data?.message ?? data?.code ?? 'Request failed';
    const error = new Error(message) as ApiError;
    error.status = response.status;
    error.payload = data;
    throw error;
  }

  return data;
};

export const api = {
  login: (username: string, password: string) =>
    fetchJson('/v1/admin/auth/login', {
      method: 'POST',
      body: JSON.stringify({ username, password })
    }),
  logout: () => fetchJson('/v1/admin/auth/logout', { method: 'POST' }),
  me: () => fetchJson('/v1/admin/auth/me'),
  services: () => fetchJson('/v1/admin/services'),
  updateServiceConfig: (service: string, configJson: unknown, expectedVersion?: number) => {
    const body: Record<string, unknown> = { config_json: configJson };
    if (typeof expectedVersion === 'number') {
      body.expected_version = expectedVersion;
    }
    return fetchJson(`/v1/admin/services/${service}/config`, {
      method: 'PUT',
      body: JSON.stringify(body)
    });
  },
  subscriptionRequests: (status?: string) =>
    fetchJson(`/v1/admin/subscription-requests${status ? `?status=${status}` : ''}`),
  approveRequest: (requestId: string, reviewNote?: string) =>
    fetchJson(`/v1/admin/subscription-requests/${requestId}/approve`, {
      method: 'POST',
      body: JSON.stringify({ review_note: reviewNote ?? null })
    }),
  rejectRequest: (requestId: string, reviewNote?: string) =>
    fetchJson(`/v1/admin/subscription-requests/${requestId}/reject`, {
      method: 'POST',
      body: JSON.stringify({ review_note: reviewNote ?? null })
    }),
  nodeSubscriptions: () => fetchJson('/v1/admin/node-subscriptions'),
  updateNodeSubscription: (topicId: string, enabled: boolean) =>
    fetchJson(`/v1/admin/node-subscriptions/${topicId}`, {
      method: 'PUT',
      body: JSON.stringify({ enabled })
    }),
  policies: () => fetchJson('/v1/admin/policies'),
  createPolicy: (payload: {
    policy_type: string;
    version: string;
    locale: string;
    title: string;
    content_md: string;
  }) => fetchJson('/v1/admin/policies', { method: 'POST', body: JSON.stringify(payload) }),
  updatePolicy: (policyId: string, payload: { title: string; content_md: string }) =>
    fetchJson(`/v1/admin/policies/${policyId}`, { method: 'PUT', body: JSON.stringify(payload) }),
  publishPolicy: (policyId: string, effectiveAt?: number) =>
    fetchJson(`/v1/admin/policies/${policyId}/publish`, {
      method: 'POST',
      body: JSON.stringify(effectiveAt ? { effective_at: effectiveAt } : {})
    }),
  makeCurrentPolicy: (policyId: string) =>
    fetchJson(`/v1/admin/policies/${policyId}/make-current`, { method: 'POST', body: JSON.stringify({}) }),
  auditLogs: (query?: { action?: string; target?: string; since?: number; limit?: number }) => {
    const params = new URLSearchParams();
    if (query?.action) {
      params.set('action', query.action);
    }
    if (query?.target) {
      params.set('target', query.target);
    }
    if (typeof query?.since === 'number') {
      params.set('since', String(query.since));
    }
    if (typeof query?.limit === 'number') {
      params.set('limit', String(query.limit));
    }
    const queryString = params.toString();
    return fetchJson(`/v1/admin/audit-logs${queryString ? `?${queryString}` : ''}`);
  },
  plans: () => fetchJson('/v1/admin/plans'),
  createPlan: (payload: { plan_id: string; name: string; is_active: boolean; limits: Array<{ metric: string; window: string; limit: number }> }) =>
    fetchJson('/v1/admin/plans', { method: 'POST', body: JSON.stringify(payload) }),
  updatePlan: (planId: string, payload: { name: string; is_active: boolean; limits: Array<{ metric: string; window: string; limit: number }> }) =>
    fetchJson(`/v1/admin/plans/${planId}`, { method: 'PUT', body: JSON.stringify({ ...payload, plan_id: planId }) }),
  subscriptions: (pubkey?: string) =>
    fetchJson(`/v1/admin/subscriptions${pubkey ? `?pubkey=${pubkey}` : ''}`),
  updateSubscription: (pubkey: string, payload: { plan_id: string; status: string }) =>
    fetchJson(`/v1/admin/subscriptions/${pubkey}`, { method: 'PUT', body: JSON.stringify(payload) }),
  usage: (pubkey: string, metric?: string, days?: number) => {
    const params = new URLSearchParams({ pubkey });
    if (metric) {
      params.set('metric', metric);
    }
    if (typeof days === 'number') {
      params.set('days', String(days));
    }
    return fetchJson(`/v1/admin/usage?${params.toString()}`);
  },
  moderationRules: (enabled?: boolean) => {
    const params = new URLSearchParams();
    if (typeof enabled === 'boolean') {
      params.set('enabled', enabled ? 'true' : 'false');
    }
    const query = params.toString();
    return fetchJson(`/v1/admin/moderation/rules${query ? `?${query}` : ''}`);
  },
  createModerationRule: (payload: {
    name: string;
    description?: string | null;
    is_enabled?: boolean;
    priority?: number;
    conditions: unknown;
    action: unknown;
  }) =>
    fetchJson('/v1/admin/moderation/rules', {
      method: 'POST',
      body: JSON.stringify(payload)
    }),
  updateModerationRule: (ruleId: string, payload: {
    name: string;
    description?: string | null;
    is_enabled?: boolean;
    priority?: number;
    conditions: unknown;
    action: unknown;
  }) =>
    fetchJson(`/v1/admin/moderation/rules/${ruleId}`, {
      method: 'PUT',
      body: JSON.stringify(payload)
    }),
  deleteModerationRule: (ruleId: string) =>
    fetchJson(`/v1/admin/moderation/rules/${ruleId}`, { method: 'DELETE' }),
  moderationReports: (query?: { target?: string; reporter_pubkey?: string; since?: number; limit?: number }) => {
    const params = new URLSearchParams();
    if (query?.target) {
      params.set('target', query.target);
    }
    if (query?.reporter_pubkey) {
      params.set('reporter_pubkey', query.reporter_pubkey);
    }
    if (typeof query?.since === 'number') {
      params.set('since', String(query.since));
    }
    if (typeof query?.limit === 'number') {
      params.set('limit', String(query.limit));
    }
    const queryString = params.toString();
    return fetchJson(`/v1/admin/moderation/reports${queryString ? `?${queryString}` : ''}`);
  },
  moderationLabels: (query?: { target?: string; topic?: string; since?: number; limit?: number }) => {
    const params = new URLSearchParams();
    if (query?.target) {
      params.set('target', query.target);
    }
    if (query?.topic) {
      params.set('topic', query.topic);
    }
    if (typeof query?.since === 'number') {
      params.set('since', String(query.since));
    }
    if (typeof query?.limit === 'number') {
      params.set('limit', String(query.limit));
    }
    const queryString = params.toString();
    return fetchJson(`/v1/admin/moderation/labels${queryString ? `?${queryString}` : ''}`);
  },
  createManualLabel: (payload: {
    target: string;
    label: string;
    confidence?: number | null;
    exp: number;
    policy_url: string;
    policy_ref: string;
    topic_id?: string | null;
  }) =>
    fetchJson('/v1/admin/moderation/labels', {
      method: 'POST',
      body: JSON.stringify(payload)
    })
};
