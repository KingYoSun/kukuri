import { useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import { StatusBadge } from '../components/StatusBadge';
import { api } from '../lib/api';
import { normalizeConnectedNode } from '../lib/bootstrap';
import { errorToMessage } from '../lib/errorHandler';
import { formatTimestamp } from '../lib/format';
import type { NodeSubscription } from '../lib/types';

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
    return {
      payload: {
        retention_days: null,
        max_events: null,
        max_bytes: null,
        allow_backfill: draft.allowBackfill
      },
      error: retention.error
    };
  }

  const maxEvents = parseInteger(draft.maxEvents, 'Max events', 1);
  if (maxEvents.error) {
    return {
      payload: {
        retention_days: retention.value,
        max_events: null,
        max_bytes: null,
        allow_backfill: draft.allowBackfill
      },
      error: maxEvents.error
    };
  }

  const maxBytes = parseInteger(draft.maxBytes, 'Max bytes', 1);
  if (maxBytes.error) {
    return {
      payload: {
        retention_days: retention.value,
        max_events: maxEvents.value,
        max_bytes: null,
        allow_backfill: draft.allowBackfill
      },
      error: maxBytes.error
    };
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

export const RelayPage = () => {
  const queryClient = useQueryClient();
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

  const nodeSubsQuery = useQuery<NodeSubscription[]>({
    queryKey: ['nodeSubscriptions'],
    queryFn: api.nodeSubscriptions
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
    const connectedUserTotal = nodeSubscriptions.reduce((total, subscription) => {
      if (
        typeof subscription.connected_user_count === 'number' &&
        Number.isFinite(subscription.connected_user_count)
      ) {
        return total + subscription.connected_user_count;
      }
      return total + (subscription.connected_users ?? []).length;
    }, 0);
    return { topicCount, connectedNodeTotal, connectedUserTotal };
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
          <h1>Relay</h1>
          <p>Manage topic subscriptions, ingest policies, and per-topic connected users.</p>
        </div>
      </div>

      <div className="card">
        <div className="row">
          <div>
            <h3>Topic Subscriptions</h3>
            <p>Relay connections are shown as node_id@host:port.</p>
          </div>
          <div className="muted">
            Topics: {nodeSubscriptionSummary.topicCount} | Connected nodes:{' '}
            {nodeSubscriptionSummary.connectedNodeTotal} | Connected users:{' '}
            {nodeSubscriptionSummary.connectedUserTotal}
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
            {createNodeSubscriptionMutation.isPending ? 'Adding...' : 'Add topic subscription'}
          </button>
          {createNodeSubscriptionError && <div className="notice">{createNodeSubscriptionError}</div>}
        </div>
        {nodeSubsQuery.isLoading && <div className="notice">Loading node topics...</div>}
        {nodeSubsQuery.error && <div className="notice">{errorToMessage(nodeSubsQuery.error)}</div>}
        <table className="table">
          <thead>
            <tr>
              <th>Topic</th>
              <th>Status</th>
              <th>Ref Count</th>
              <th>Connected Nodes</th>
              <th>Node Endpoints</th>
              <th>Connected Users</th>
              <th>User Pubkeys</th>
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
              const connectedUsers = Array.from(
                new Set((sub.connected_users ?? []).map((user) => user.trim()).filter((user) => user !== ''))
              ).sort();
              const connectedNodeCount =
                typeof sub.connected_node_count === 'number' && Number.isFinite(sub.connected_node_count)
                  ? sub.connected_node_count
                  : connectedNodes.length;
              const connectedUserCount =
                typeof sub.connected_user_count === 'number' && Number.isFinite(sub.connected_user_count)
                  ? sub.connected_user_count
                  : connectedUsers.length;
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
                  <td>{connectedUserCount}</td>
                  <td>
                    {connectedUsers.length === 0 ? (
                      <span className="muted">No connected users</span>
                    ) : (
                      <div className="stack">
                        {connectedUsers.map((userPubkey) => (
                          <code key={`${sub.topic_id}-${userPubkey}`}>{userPubkey}</code>
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
                <td colSpan={13}>No node subscriptions configured.</td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </>
  );
};
