import { useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { z } from 'zod';

import { api } from '../lib/api';
import { errorToMessage } from '../lib/errorHandler';
import { formatJson, formatTimestamp } from '../lib/format';
import { subscriptionsQueryOptions } from '../lib/subscriptionsQuery';
import type { Plan, PlanLimit, SubscriptionRequest, SubscriptionRow, UsageRow } from '../lib/types';
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

  const requestsQuery = useQuery<SubscriptionRequest[]>({
    queryKey: ['subscriptionRequests', requestFilter],
    queryFn: () =>
      api.subscriptionRequests(requestFilter === 'all' ? undefined : requestFilter)
  });

  const plansQuery = useQuery<Plan[]>({
    queryKey: ['plans'],
    queryFn: api.plans
  });

  const subscriptionsQuery = useQuery<SubscriptionRow[]>({
    ...subscriptionsQueryOptions(subscriptionFilter)
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

  return (
    <>
      <div className="hero">
        <div>
          <h1>Subscriptions</h1>
          <p>Approve requests, manage plans, and track user subscriptions and usage.</p>
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
