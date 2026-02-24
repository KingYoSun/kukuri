import { useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { z } from 'zod';

import { api } from '../lib/api';
import { normalizeConnectedNode } from '../lib/bootstrap';
import { errorToMessage } from '../lib/errorHandler';
import { formatJson, formatTimestamp } from '../lib/format';
import type {
  NodeSubscription,
  Plan,
  PlanLimit,
  SubscriptionRequest,
  SubscriptionRow,
  UsageRow
} from '../lib/types';
import { StatusBadge } from '../components/StatusBadge';

const planLimitSchema = z.object({
  metric: z.string().min(1),
  window: z.string().min(1),
  limit: z.number().int().nonnegative()
});

const planSchema = z.object({
  plan_id: z.string().min(1, 'Plan ID is required'),
  name: z.string().min(1, 'Name is required'),
  is_active: z.boolean(),
  limits: z.array(planLimitSchema).min(1, 'At least one limit is required')
});

const subscriptionUpdateSchema = z.object({
  pubkey: z.string().min(1, 'Pubkey is required'),
  plan_id: z.string().min(1, 'Plan ID is required'),
  status: z.enum(['active', 'paused', 'cancelled', 'disabled'])
});

type NodePolicyDraft = {
  retentionDays: string;
  maxEvents: string;
  maxBytes: string;
  allowBackfill: boolean;
};

type NodeIngestPolicyPayload = {
  retention_days: number | null;
  max_events: number | null;
  max_bytes: number | null;
  allow_backfill: boolean;
};

type NodeSubscriptionCreateForm = {
  topicId: string;
  enabled: boolean;
  policy: NodePolicyDraft;
};

const toNodePolicyDraft = (policy: NodeSubscription['ingest_policy']): NodePolicyDraft => ({
  retentionDays:
    typeof policy?.retention_days === 'number' && Number.isFinite(policy.retention_days)
      ? String(policy.retention_days)
      : '',
  maxEvents:
    typeof policy?.max_events === 'number' && Number.isFinite(policy.max_events)
      ? String(policy.max_events)
      : '',
  maxBytes:
    typeof policy?.max_bytes === 'number' && Number.isFinite(policy.max_bytes)
      ? String(policy.max_bytes)
      : '',
  allowBackfill: policy?.allow_backfill ?? true
});

const parseNodePolicyDraft = (
  draft: NodePolicyDraft
): { payload: NodeIngestPolicyPayload; error: string | null } => {
  const parseInteger = (
    raw: string,
    label: string,
    min: number
  ): { value: number | null; error: string | null } => {
    const trimmed = raw.trim();
    if (trimmed === '') {
      return { value: null, error: null };
    }
    const parsed = Number(trimmed);
    if (!Number.isInteger(parsed) || !Number.isFinite(parsed) || parsed < min) {
      return { value: null, error: `${label} must be an integer >= ${min}.` };
    }
    return { value: parsed, error: null };
  };

  const retention = parseInteger(draft.retentionDays, 'Retention days', 0);
  if (retention.error) {
    return { payload: { retention_days: null, max_events: null, max_bytes: null, allow_backfill: draft.allowBackfill }, error: retention.error };
  }
  const maxEvents = parseInteger(draft.maxEvents, 'Max events', 1);
  if (maxEvents.error) {
    return { payload: { retention_days: retention.value, max_events: null, max_bytes: null, allow_backfill: draft.allowBackfill }, error: maxEvents.error };
  }
  const maxBytes = parseInteger(draft.maxBytes, 'Max bytes', 1);
  if (maxBytes.error) {
    return { payload: { retention_days: retention.value, max_events: maxEvents.value, max_bytes: null, allow_backfill: draft.allowBackfill }, error: maxBytes.error };
  }

  return {
    payload: {
      retention_days: retention.value,
      max_events: maxEvents.value,
      max_bytes: maxBytes.value,
      allow_backfill: draft.allowBackfill
    },
    error: null
  };
};

export const SubscriptionsPage = () => {
  const queryClient = useQueryClient();
  const [requestFilter, setRequestFilter] = useState('pending');
  const [reviewNotes, setReviewNotes] = useState<Record<string, string>>({});
  const [planForm, setPlanForm] = useState({
    plan_id: '',
    name: '',
    is_active: true,
    limitsText: formatJson([{ metric: 'index.search_requests', window: 'day', limit: 100 }])
  });
  const [planMode, setPlanMode] = useState<'create' | 'edit'>('create');
  const [planError, setPlanError] = useState<string | null>(null);
  const [subscriptionForm, setSubscriptionForm] = useState({
    pubkey: '',
    plan_id: '',
    status: 'active'
  });
  const [subscriptionUpdateError, setSubscriptionUpdateError] = useState<string | null>(null);
  const [subscriptionFilter, setSubscriptionFilter] = useState('');
  const [usageForm, setUsageForm] = useState({ pubkey: '', metric: '', days: '30' });
  const [usageError, setUsageError] = useState<string | null>(null);
  const [nodePolicyDrafts, setNodePolicyDrafts] = useState<Record<string, NodePolicyDraft>>({});
  const [nodePolicyErrors, setNodePolicyErrors] = useState<Record<string, string | null>>({});
  const [newNodeSubscriptionForm, setNewNodeSubscriptionForm] = useState<NodeSubscriptionCreateForm>({
    topicId: '',
    enabled: true,
    policy: {
      retentionDays: '',
      maxEvents: '',
      maxBytes: '',
      allowBackfill: true
    }
  });
  const [createNodeSubscriptionError, setCreateNodeSubscriptionError] = useState<string | null>(
    null
  );
  const [deleteNodeSubscriptionErrors, setDeleteNodeSubscriptionErrors] = useState<
    Record<string, string | null>
  >({});

  const requestsQuery = useQuery<SubscriptionRequest[]>({
    queryKey: ['subscriptionRequests', requestFilter],
    queryFn: () =>
      api.subscriptionRequests(requestFilter === 'all' ? undefined : requestFilter)
  });

  const nodeSubsQuery = useQuery<NodeSubscription[]>({
    queryKey: ['nodeSubscriptions'],
    queryFn: api.nodeSubscriptions
  });

  const plansQuery = useQuery<Plan[]>({
    queryKey: ['plans'],
    queryFn: api.plans
  });

  const subscriptionsQuery = useQuery<SubscriptionRow[]>({
    queryKey: ['subscriptions', subscriptionFilter],
    queryFn: () => api.subscriptions(subscriptionFilter || undefined)
  });

  const usageQuery = useQuery<UsageRow[]>({
    queryKey: ['usage', usageForm.pubkey, usageForm.metric, usageForm.days],
    queryFn: () =>
      api.usage(
        usageForm.pubkey,
        usageForm.metric || undefined,
        usageForm.days ? Number(usageForm.days) : undefined
      ),
    enabled: false
  });

  const approveMutation = useMutation({
    mutationFn: ({ requestId, note }: { requestId: string; note?: string }) =>
      api.approveRequest(requestId, note),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['subscriptionRequests'] });
      queryClient.invalidateQueries({ queryKey: ['nodeSubscriptions'] });
    }
  });

  const rejectMutation = useMutation({
    mutationFn: ({ requestId, note }: { requestId: string; note?: string }) =>
      api.rejectRequest(requestId, note),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['subscriptionRequests'] });
    }
  });

  const nodeSubscriptionMutation = useMutation({
    mutationFn: ({
      topicId,
      enabled,
      ingestPolicy
    }: {
      topicId: string;
      enabled: boolean;
      ingestPolicy?: NodeIngestPolicyPayload;
    }) =>
      api.updateNodeSubscription(topicId, {
        enabled,
        ingest_policy: ingestPolicy
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['nodeSubscriptions'] });
    },
    onError: (err, variables) => {
      setNodePolicyErrors((prev) => ({
        ...prev,
        [variables.topicId]: errorToMessage(err)
      }));
    }
  });

  const createNodeSubscriptionMutation = useMutation({
    mutationFn: (payload: {
      topic_id: string;
      enabled: boolean;
      ingest_policy: NodeIngestPolicyPayload;
    }) => api.createNodeSubscription(payload),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['nodeSubscriptions'] });
      setCreateNodeSubscriptionError(null);
      setNewNodeSubscriptionForm({
        topicId: '',
        enabled: true,
        policy: {
          retentionDays: '',
          maxEvents: '',
          maxBytes: '',
          allowBackfill: true
        }
      });
    },
    onError: (err) => {
      setCreateNodeSubscriptionError(errorToMessage(err));
    }
  });

  const deleteNodeSubscriptionMutation = useMutation({
    mutationFn: (topicId: string) => api.deleteNodeSubscription(topicId),
    onSuccess: (_payload, topicId) => {
      queryClient.invalidateQueries({ queryKey: ['nodeSubscriptions'] });
      setDeleteNodeSubscriptionErrors((prev) => ({
        ...prev,
        [topicId]: null
      }));
    },
    onError: (err, topicId) => {
      setDeleteNodeSubscriptionErrors((prev) => ({
        ...prev,
        [topicId]: errorToMessage(err)
      }));
    }
  });

  const planMutation = useMutation({
    mutationFn: (payload: { mode: 'create' | 'edit'; plan: Plan }) => {
      if (payload.mode === 'edit') {
        return api.updatePlan(payload.plan.plan_id, {
          name: payload.plan.name,
          is_active: payload.plan.is_active,
          limits: payload.plan.limits
        });
      }
      return api.createPlan(payload.plan);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['plans'] });
      setPlanError(null);
    },
    onError: (err) => {
      setPlanError(errorToMessage(err));
    }
  });

  const subscriptionMutation = useMutation({
    mutationFn: (payload: { pubkey: string; plan_id: string; status: string }) =>
      api.updateSubscription(payload.pubkey, {
        plan_id: payload.plan_id,
        status: payload.status
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['subscriptions'] });
      setSubscriptionUpdateError(null);
    },
    onError: (err) => {
      setSubscriptionUpdateError(errorToMessage(err));
    }
  });

  const requestList = requestsQuery.data ?? [];
  const planOptions = plansQuery.data ?? [];

  const subscriptionError = subscriptionsQuery.error
    ? errorToMessage(subscriptionsQuery.error)
    : null;

  const usageResult = usageQuery.data ?? [];

  const parseLimits = () => {
    try {
      const parsed = JSON.parse(planForm.limitsText) as unknown;
      const check = z.array(planLimitSchema).safeParse(parsed);
      if (!check.success) {
        return {
          error: check.error.issues[0]?.message ?? 'Invalid limits format',
          limits: null
        };
      }
      return { error: null, limits: check.data as PlanLimit[] };
    } catch {
      return { error: 'Limits must be valid JSON.', limits: null };
    }
  };

  const submitPlan = () => {
    setPlanError(null);
    const { error, limits } = parseLimits();
    if (error || !limits) {
      setPlanError(error ?? 'Invalid limits');
      return;
    }
    const parsed = planSchema.safeParse({
      plan_id: planForm.plan_id,
      name: planForm.name,
      is_active: planForm.is_active,
      limits
    });
    if (!parsed.success) {
      setPlanError(parsed.error.issues[0]?.message ?? 'Invalid plan data');
      return;
    }
    planMutation.mutate({ mode: planMode, plan: parsed.data });
  };

  const editPlan = (plan: Plan) => {
    setPlanMode('edit');
    setPlanForm({
      plan_id: plan.plan_id,
      name: plan.name,
      is_active: plan.is_active,
      limitsText: formatJson(plan.limits)
    });
  };

  const resetPlanForm = () => {
    setPlanMode('create');
    setPlanForm({
      plan_id: '',
      name: '',
      is_active: true,
      limitsText: formatJson([{ metric: 'index.search_requests', window: 'day', limit: 100 }])
    });
    setPlanError(null);
  };

  const submitSubscriptionUpdate = () => {
    setSubscriptionUpdateError(null);
    const parsed = subscriptionUpdateSchema.safeParse(subscriptionForm);
    if (!parsed.success) {
      setSubscriptionUpdateError(
        parsed.error.issues[0]?.message ?? 'Invalid subscription data'
      );
      return;
    }
    subscriptionMutation.mutate(parsed.data);
  };

  const submitUsageQuery = async () => {
    setUsageError(null);
    if (!usageForm.pubkey) {
      setUsageError('Pubkey is required for usage lookup.');
      return;
    }
    try {
      await usageQuery.refetch();
    } catch (err) {
      setUsageError(errorToMessage(err));
    }
  };

  const requestStatusCounts = useMemo(() => {
    const counts = new Map<string, number>();
    requestList.forEach((req) => {
      counts.set(req.status, (counts.get(req.status) ?? 0) + 1);
    });
    return counts;
  }, [requestList]);

  const nodeSubscriptions = nodeSubsQuery.data ?? [];
  const nodeSubscriptionSummary = useMemo(() => {
    const topicCount = nodeSubscriptions.length;
    const connectedNodeTotal = nodeSubscriptions.reduce((total, subscription) => {
      if (
        typeof subscription.connected_node_count === 'number' &&
        Number.isFinite(subscription.connected_node_count)
      ) {
        return total + subscription.connected_node_count;
      }
      return total + (subscription.connected_nodes ?? []).length;
    }, 0);
    return { topicCount, connectedNodeTotal };
  }, [nodeSubscriptions]);

  const saveNodePolicy = (subscription: NodeSubscription) => {
    const draft =
      nodePolicyDrafts[subscription.topic_id] ?? toNodePolicyDraft(subscription.ingest_policy);
    const parsed = parseNodePolicyDraft(draft);
    if (parsed.error) {
      setNodePolicyErrors((prev) => ({
        ...prev,
        [subscription.topic_id]: parsed.error
      }));
      return;
    }
    setNodePolicyErrors((prev) => ({
      ...prev,
      [subscription.topic_id]: null
    }));
    nodeSubscriptionMutation.mutate({
      topicId: subscription.topic_id,
      enabled: subscription.enabled,
      ingestPolicy: parsed.payload
    });
  };

  const toggleNodeSubscription = (subscription: NodeSubscription) => {
    const parsed = parseNodePolicyDraft(toNodePolicyDraft(subscription.ingest_policy));
    nodeSubscriptionMutation.mutate({
      topicId: subscription.topic_id,
      enabled: !subscription.enabled,
      ingestPolicy: parsed.payload
    });
  };

  const submitNodeSubscriptionCreate = () => {
    setCreateNodeSubscriptionError(null);
    const topicId = newNodeSubscriptionForm.topicId.trim();
    if (topicId === '') {
      setCreateNodeSubscriptionError('Topic ID is required.');
      return;
    }

    const parsed = parseNodePolicyDraft(newNodeSubscriptionForm.policy);
    if (parsed.error) {
      setCreateNodeSubscriptionError(parsed.error);
      return;
    }

    createNodeSubscriptionMutation.mutate({
      topic_id: topicId,
      enabled: newNodeSubscriptionForm.enabled,
      ingest_policy: parsed.payload
    });
  };

  const removeNodeSubscription = (topicId: string) => {
    setDeleteNodeSubscriptionErrors((prev) => ({
      ...prev,
      [topicId]: null
    }));
    deleteNodeSubscriptionMutation.mutate(topicId);
  };

  return (
    <>
      <div className="hero">
        <div>
          <h1>Subscriptions</h1>
          <p>Approve requests, manage node-level subscriptions, and plan limits.</p>
        </div>
      </div>

      <div className="card">
        <div className="row">
          <div>
            <h3>Subscription Requests</h3>
            <p>Pending approvals and history.</p>
          </div>
          <select
            value={requestFilter}
            onChange={(event) => setRequestFilter(event.target.value)}
          >
            <option value="pending">Pending</option>
            <option value="approved">Approved</option>
            <option value="rejected">Rejected</option>
            <option value="all">All</option>
          </select>
        </div>
        {requestsQuery.isLoading && <div className="notice">Loading requests...</div>}
        {requestsQuery.error && (
          <div className="notice">{errorToMessage(requestsQuery.error)}</div>
        )}
        <div className="muted">
          {Array.from(requestStatusCounts.entries())
            .map(([status, count]) => `${status}: ${count}`)
            .join(' | ')}
        </div>
        <div className="stack">
          {requestList.map((request) => {
            const note = reviewNotes[request.request_id] ?? '';
            const busy = approveMutation.isPending || rejectMutation.isPending;
            return (
              <div key={request.request_id} className="card sub-card">
                <div className="row">
                  <div>
                    <strong>{request.topic_id}</strong>
                    <div className="muted">{request.requester_pubkey}</div>
                  </div>
                  <StatusBadge status={request.status} />
                </div>
                <pre className="code-block">{formatJson(request.requested_services)}</pre>
                <div className="muted">
                  Created {formatTimestamp(request.created_at)} | Reviewed{' '}
                  {formatTimestamp(request.reviewed_at ?? null)}
                </div>
                <div className="field">
                  <label>Review note</label>
                  <input
                    value={note}
                    onChange={(event) =>
                      setReviewNotes((prev) => ({
                        ...prev,
                        [request.request_id]: event.target.value
                      }))
                    }
                  />
                </div>
                <div className="row">
                  <button
                    className="button"
                    disabled={busy}
                    onClick={() =>
                      approveMutation.mutate({ requestId: request.request_id, note })
                    }
                  >
                    Approve
                  </button>
                  <button
                    className="button secondary"
                    disabled={busy}
                    onClick={() =>
                      rejectMutation.mutate({ requestId: request.request_id, note })
                    }
                  >
                    Reject
                  </button>
                </div>
              </div>
            );
          })}
          {requestList.length === 0 && <div className="notice">No requests found.</div>}
        </div>
      </div>

      <div className="card">
        <div className="row">
          <div>
            <h3>Node Subscriptions</h3>
            <p>Relay connections are shown as node_id@host:port.</p>
          </div>
          <div className="muted">
            Topics: {nodeSubscriptionSummary.topicCount} | Connected nodes:{' '}
            {nodeSubscriptionSummary.connectedNodeTotal}
          </div>
        </div>
        <div className="card sub-card">
          <h3>Add topic subscription</h3>
          <div className="grid">
            <div className="field">
              <label>Topic ID</label>
              <input
                aria-label="New topic ID"
                value={newNodeSubscriptionForm.topicId}
                onChange={(event) => {
                  setCreateNodeSubscriptionError(null);
                  setNewNodeSubscriptionForm((prev) => ({
                    ...prev,
                    topicId: event.target.value
                  }));
                }}
                placeholder="kukuri:topic:example"
              />
            </div>
            <div className="field">
              <label>Enabled</label>
              <select
                aria-label="New topic enabled"
                value={newNodeSubscriptionForm.enabled ? 'enabled' : 'disabled'}
                onChange={(event) =>
                  setNewNodeSubscriptionForm((prev) => ({
                    ...prev,
                    enabled: event.target.value === 'enabled'
                  }))
                }
              >
                <option value="enabled">enabled</option>
                <option value="disabled">disabled</option>
              </select>
            </div>
            <div className="field">
              <label>Retention days</label>
              <input
                aria-label="New retention days"
                value={newNodeSubscriptionForm.policy.retentionDays}
                onChange={(event) =>
                  setNewNodeSubscriptionForm((prev) => ({
                    ...prev,
                    policy: {
                      ...prev.policy,
                      retentionDays: event.target.value
                    }
                  }))
                }
                placeholder="global"
              />
            </div>
            <div className="field">
              <label>Max events</label>
              <input
                aria-label="New max events"
                value={newNodeSubscriptionForm.policy.maxEvents}
                onChange={(event) =>
                  setNewNodeSubscriptionForm((prev) => ({
                    ...prev,
                    policy: {
                      ...prev.policy,
                      maxEvents: event.target.value
                    }
                  }))
                }
                placeholder="unlimited"
              />
            </div>
            <div className="field">
              <label>Max bytes</label>
              <input
                aria-label="New max bytes"
                value={newNodeSubscriptionForm.policy.maxBytes}
                onChange={(event) =>
                  setNewNodeSubscriptionForm((prev) => ({
                    ...prev,
                    policy: {
                      ...prev.policy,
                      maxBytes: event.target.value
                    }
                  }))
                }
                placeholder="unlimited"
              />
            </div>
            <div className="field">
              <label>Backfill</label>
              <select
                aria-label="New backfill"
                value={newNodeSubscriptionForm.policy.allowBackfill ? 'enabled' : 'disabled'}
                onChange={(event) =>
                  setNewNodeSubscriptionForm((prev) => ({
                    ...prev,
                    policy: {
                      ...prev.policy,
                      allowBackfill: event.target.value === 'enabled'
                    }
                  }))
                }
              >
                <option value="enabled">enabled</option>
                <option value="disabled">disabled</option>
              </select>
            </div>
          </div>
          <button
            className="button"
            onClick={submitNodeSubscriptionCreate}
            disabled={createNodeSubscriptionMutation.isPending}
          >
            {createNodeSubscriptionMutation.isPending
              ? 'Adding...'
              : 'Add topic subscription'}
          </button>
          {createNodeSubscriptionError && (
            <div className="notice">{createNodeSubscriptionError}</div>
          )}
        </div>
        {nodeSubsQuery.isLoading && <div className="notice">Loading node topics...</div>}
        {nodeSubsQuery.error && (
          <div className="notice">{errorToMessage(nodeSubsQuery.error)}</div>
        )}
        <table className="table">
          <thead>
            <tr>
              <th>Topic</th>
              <th>Status</th>
              <th>Ref Count</th>
              <th>Connected</th>
              <th>Connected Nodes</th>
              <th>Retention Days</th>
              <th>Max Events</th>
              <th>Max Bytes</th>
              <th>Backfill</th>
              <th>Updated</th>
              <th>Actions</th>
            </tr>
          </thead>
          <tbody>
            {nodeSubscriptions.map((sub) => {
              const draft = nodePolicyDrafts[sub.topic_id] ?? toNodePolicyDraft(sub.ingest_policy);
              const connectedNodes = Array.from(
                new Set((sub.connected_nodes ?? []).map(normalizeConnectedNode))
              ).sort();
              const connectedNodeCount =
                typeof sub.connected_node_count === 'number' &&
                Number.isFinite(sub.connected_node_count)
                  ? sub.connected_node_count
                  : connectedNodes.length;
              const rowError = nodePolicyErrors[sub.topic_id];
              const deleteError = deleteNodeSubscriptionErrors[sub.topic_id];
              return (
                <tr key={sub.topic_id}>
                  <td>{sub.topic_id}</td>
                  <td>
                    <StatusBadge status={sub.enabled ? 'enabled' : 'disabled'} />
                  </td>
                  <td>{sub.ref_count}</td>
                  <td>{connectedNodeCount}</td>
                  <td>
                    {connectedNodes.length === 0 ? (
                      <span className="muted">No connected nodes</span>
                    ) : (
                      <div className="stack">
                        {connectedNodes.map((node) => (
                          <code key={`${sub.topic_id}-${node}`}>{node}</code>
                        ))}
                      </div>
                    )}
                  </td>
                  <td>
                    <input
                      aria-label={`Retention days ${sub.topic_id}`}
                      value={draft.retentionDays}
                      onChange={(event) =>
                        setNodePolicyDrafts((prev) => ({
                          ...prev,
                          [sub.topic_id]: {
                            ...draft,
                            retentionDays: event.target.value
                          }
                        }))
                      }
                      placeholder="global"
                    />
                  </td>
                  <td>
                    <input
                      aria-label={`Max events ${sub.topic_id}`}
                      value={draft.maxEvents}
                      onChange={(event) =>
                        setNodePolicyDrafts((prev) => ({
                          ...prev,
                          [sub.topic_id]: {
                            ...draft,
                            maxEvents: event.target.value
                          }
                        }))
                      }
                      placeholder="unlimited"
                    />
                  </td>
                  <td>
                    <input
                      aria-label={`Max bytes ${sub.topic_id}`}
                      value={draft.maxBytes}
                      onChange={(event) =>
                        setNodePolicyDrafts((prev) => ({
                          ...prev,
                          [sub.topic_id]: {
                            ...draft,
                            maxBytes: event.target.value
                          }
                        }))
                      }
                      placeholder="unlimited"
                    />
                  </td>
                  <td>
                    <select
                      aria-label={`Backfill ${sub.topic_id}`}
                      value={draft.allowBackfill ? 'enabled' : 'disabled'}
                      onChange={(event) =>
                        setNodePolicyDrafts((prev) => ({
                          ...prev,
                          [sub.topic_id]: {
                            ...draft,
                            allowBackfill: event.target.value === 'enabled'
                          }
                        }))
                      }
                    >
                      <option value="enabled">enabled</option>
                      <option value="disabled">disabled</option>
                    </select>
                  </td>
                  <td>{formatTimestamp(sub.updated_at)}</td>
                  <td>
                    <div className="row">
                      <button
                        className="button secondary"
                        onClick={() => toggleNodeSubscription(sub)}
                        disabled={nodeSubscriptionMutation.isPending}
                      >
                        Toggle
                      </button>
                      <button
                        className="button"
                        onClick={() => saveNodePolicy(sub)}
                        disabled={nodeSubscriptionMutation.isPending}
                      >
                        Save policy
                      </button>
                      <button
                        className="button secondary"
                        onClick={() => removeNodeSubscription(sub.topic_id)}
                        disabled={deleteNodeSubscriptionMutation.isPending}
                      >
                        Delete topic
                      </button>
                    </div>
                    {rowError && <div className="notice">{rowError}</div>}
                    {deleteError && <div className="notice">{deleteError}</div>}
                  </td>
                </tr>
              );
            })}
            {nodeSubscriptions.length === 0 && (
              <tr>
                <td colSpan={11}>No node subscriptions configured.</td>
              </tr>
            )}
          </tbody>
        </table>
      </div>

      <div className="card">
        <div className="row">
          <div>
            <h3>Plans</h3>
            <p>Create or update usage limits.</p>
          </div>
          {planMode === 'edit' && (
            <button className="button secondary" onClick={resetPlanForm}>
              Reset form
            </button>
          )}
        </div>
        <div className="grid">
          <div className="card">
            <div className="field">
              <label>Plan ID</label>
              <input
                value={planForm.plan_id}
                onChange={(event) =>
                  setPlanForm((prev) => ({ ...prev, plan_id: event.target.value }))
                }
                disabled={planMode === 'edit'}
              />
            </div>
            <div className="field">
              <label>Name</label>
              <input
                value={planForm.name}
                onChange={(event) =>
                  setPlanForm((prev) => ({ ...prev, name: event.target.value }))
                }
              />
            </div>
            <div className="field">
              <label>Active</label>
              <select
                value={planForm.is_active ? 'true' : 'false'}
                onChange={(event) =>
                  setPlanForm((prev) => ({
                    ...prev,
                    is_active: event.target.value === 'true'
                  }))
                }
              >
                <option value="true">Active</option>
                <option value="false">Inactive</option>
              </select>
            </div>
            <div className="field">
              <label>Limits (JSON)</label>
              <textarea
                rows={8}
                value={planForm.limitsText}
                onChange={(event) =>
                  setPlanForm((prev) => ({ ...prev, limitsText: event.target.value }))
                }
              />
            </div>
            {planError && <div className="notice">{planError}</div>}
            <button className="button" onClick={submitPlan} disabled={planMutation.isPending}>
              {planMutation.isPending
                ? 'Saving...'
                : planMode === 'edit'
                  ? 'Update plan'
                  : 'Create plan'}
            </button>
          </div>
          <div className="card">
            <h3>Existing Plans</h3>
            {plansQuery.isLoading && <div className="notice">Loading plans...</div>}
            {plansQuery.error && (
              <div className="notice">{errorToMessage(plansQuery.error)}</div>
            )}
            <div className="stack">
              {planOptions.map((plan) => (
                <div key={plan.plan_id} className="card sub-card">
                  <div className="row">
                    <div>
                      <strong>{plan.plan_id}</strong>
                      <div className="muted">{plan.name}</div>
                    </div>
                    <StatusBadge status={plan.is_active ? 'active' : 'inactive'} />
                  </div>
                  <pre className="code-block">{formatJson(plan.limits)}</pre>
                  <button className="button secondary" onClick={() => editPlan(plan)}>
                    Edit
                  </button>
                </div>
              ))}
              {planOptions.length === 0 && <div className="notice">No plans found.</div>}
            </div>
          </div>
        </div>
      </div>

      <div className="card">
        <h3>Subscriptions</h3>
        <div className="row">
          <div className="field">
            <label>Filter by pubkey (optional)</label>
            <input
              value={subscriptionFilter}
              onChange={(event) => setSubscriptionFilter(event.target.value)}
              placeholder="npub or hex"
            />
          </div>
        </div>
        {subscriptionsQuery.isLoading && <div className="notice">Loading subscriptions...</div>}
        {subscriptionError && <div className="notice">{subscriptionError}</div>}
        <table className="table">
          <thead>
            <tr>
              <th>Subscriber</th>
              <th>Plan</th>
              <th>Status</th>
              <th>Started</th>
              <th>Ended</th>
            </tr>
          </thead>
          <tbody>
            {(subscriptionsQuery.data ?? []).map((row) => (
              <tr key={row.subscription_id}>
                <td>{row.subscriber_pubkey}</td>
                <td>{row.plan_id}</td>
                <td>
                  <StatusBadge status={row.status} />
                </td>
                <td>{formatTimestamp(row.started_at)}</td>
                <td>{formatTimestamp(row.ended_at ?? null)}</td>
              </tr>
            ))}
            {(subscriptionsQuery.data ?? []).length === 0 && (
              <tr>
                <td colSpan={5}>No subscriptions found.</td>
              </tr>
            )}
          </tbody>
        </table>
        <div className="divider" />
        <div className="grid">
          <div className="card">
            <h3>Update Subscription</h3>
            <div className="field">
              <label>Subscriber Pubkey</label>
              <input
                value={subscriptionForm.pubkey}
                onChange={(event) =>
                  setSubscriptionForm((prev) => ({ ...prev, pubkey: event.target.value }))
                }
              />
            </div>
            <div className="field">
              <label>Plan ID</label>
              <input
                value={subscriptionForm.plan_id}
                onChange={(event) =>
                  setSubscriptionForm((prev) => ({ ...prev, plan_id: event.target.value }))
                }
              />
            </div>
            <div className="field">
              <label>Status</label>
              <select
                value={subscriptionForm.status}
                onChange={(event) =>
                  setSubscriptionForm((prev) => ({ ...prev, status: event.target.value }))
                }
              >
                <option value="active">active</option>
                <option value="paused">paused</option>
                <option value="cancelled">cancelled</option>
                <option value="disabled">disabled</option>
              </select>
            </div>
            <button
              className="button"
              onClick={submitSubscriptionUpdate}
              disabled={subscriptionMutation.isPending}
            >
            {subscriptionMutation.isPending ? 'Saving...' : 'Apply'}
            </button>
            {subscriptionUpdateError && <div className="notice">{subscriptionUpdateError}</div>}
          </div>
          <div className="card">
            <h3>Usage</h3>
            <div className="field">
              <label>Subscriber Pubkey</label>
              <input
                value={usageForm.pubkey}
                onChange={(event) =>
                  setUsageForm((prev) => ({ ...prev, pubkey: event.target.value }))
                }
              />
            </div>
            <div className="field">
              <label>Metric (optional)</label>
              <input
                value={usageForm.metric}
                onChange={(event) =>
                  setUsageForm((prev) => ({ ...prev, metric: event.target.value }))
                }
              />
            </div>
            <div className="field">
              <label>Days (optional)</label>
              <input
                value={usageForm.days}
                onChange={(event) =>
                  setUsageForm((prev) => ({ ...prev, days: event.target.value }))
                }
              />
            </div>
            {usageError && <div className="notice">{usageError}</div>}
            <button className="button" onClick={submitUsageQuery}>
              Fetch usage
            </button>
            <table className="table">
              <thead>
                <tr>
                  <th>Metric</th>
                  <th>Day</th>
                  <th>Count</th>
                </tr>
              </thead>
              <tbody>
                {usageResult.map((row) => (
                  <tr key={`${row.metric}-${row.day}`}>
                    <td>{row.metric}</td>
                    <td>{row.day}</td>
                    <td>{row.count}</td>
                  </tr>
                ))}
                {usageResult.length === 0 && (
                  <tr>
                    <td colSpan={3}>No usage data.</td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
        </div>
      </div>
    </>
  );
};
