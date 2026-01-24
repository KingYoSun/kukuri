import { useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import { api } from '../lib/api';
import { errorToMessage } from '../lib/errorHandler';
import { formatJson, formatTimestamp } from '../lib/format';
import type { ModerationLabel, ModerationReport, ModerationRule } from '../lib/types';
import { StatusBadge } from '../components/StatusBadge';

const defaultConditions = JSON.stringify({ kinds: [1], content_keywords: ['spam'] }, null, 2);
const defaultAction = JSON.stringify(
  {
    label: 'spam',
    confidence: 0.6,
    exp_seconds: 86400,
    policy_url: 'https://example.com/policy',
    policy_ref: 'moderation-v1'
  },
  null,
  2
);

export const ModerationPage = () => {
  const queryClient = useQueryClient();
  const [mode, setMode] = useState<'create' | 'edit'>('create');
  const [editingRuleId, setEditingRuleId] = useState<string | null>(null);
  const [ruleError, setRuleError] = useState<string | null>(null);
  const [ruleForm, setRuleForm] = useState({
    name: '',
    description: '',
    priority: '0',
    is_enabled: true,
    conditions: defaultConditions,
    action: defaultAction
  });

  const [labelError, setLabelError] = useState<string | null>(null);
  const [labelTarget, setLabelTarget] = useState('');
  const [labelForm, setLabelForm] = useState({
    target: '',
    label: '',
    confidence: '',
    expiresInHours: '24',
    policy_url: '',
    policy_ref: '',
    topic_id: ''
  });

  const rulesQuery = useQuery<ModerationRule[]>({
    queryKey: ['moderation-rules'],
    queryFn: () => api.moderationRules()
  });

  const reportsQuery = useQuery<ModerationReport[]>({
    queryKey: ['moderation-reports'],
    queryFn: () => api.moderationReports({ limit: 50 })
  });

  const labelsQuery = useQuery<ModerationLabel[]>({
    queryKey: ['moderation-labels', labelTarget],
    queryFn: () =>
      api.moderationLabels({
        limit: 50,
        target: labelTarget.trim() !== '' ? labelTarget.trim() : undefined
      })
  });

  const saveRuleMutation = useMutation({
    mutationFn: (payload: {
      name: string;
      description?: string | null;
      is_enabled?: boolean;
      priority?: number;
      conditions: unknown;
      action: unknown;
    }) => {
      if (mode === 'edit' && editingRuleId) {
        return api.updateModerationRule(editingRuleId, payload);
      }
      return api.createModerationRule(payload);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['moderation-rules'] });
      setRuleError(null);
      resetRuleForm();
    },
    onError: (err) => setRuleError(errorToMessage(err))
  });

  const deleteRuleMutation = useMutation({
    mutationFn: (ruleId: string) => api.deleteModerationRule(ruleId),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['moderation-rules'] })
  });

  const manualLabelMutation = useMutation({
    mutationFn: (payload: {
      target: string;
      label: string;
      confidence?: number | null;
      exp: number;
      policy_url: string;
      policy_ref: string;
      topic_id?: string | null;
    }) => api.createManualLabel(payload),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['moderation-labels'] });
      setLabelError(null);
    },
    onError: (err) => setLabelError(errorToMessage(err))
  });

  const handleRuleSubmit = () => {
    setRuleError(null);
    const priorityValue = Number(ruleForm.priority);
    if (!ruleForm.name.trim()) {
      setRuleError('Rule name is required.');
      return;
    }
    if (Number.isNaN(priorityValue)) {
      setRuleError('Priority must be a number.');
      return;
    }

    let conditions: unknown;
    let action: unknown;
    try {
      conditions = JSON.parse(ruleForm.conditions);
      action = JSON.parse(ruleForm.action);
    } catch (err) {
      setRuleError(errorToMessage(err));
      return;
    }

    saveRuleMutation.mutate({
      name: ruleForm.name,
      description: ruleForm.description || null,
      is_enabled: ruleForm.is_enabled,
      priority: priorityValue,
      conditions,
      action
    });
  };

  const handleLabelSubmit = () => {
    setLabelError(null);
    const hours = Number(labelForm.expiresInHours);
    if (!labelForm.target.trim() || !labelForm.label.trim()) {
      setLabelError('Target and label are required.');
      return;
    }
    if (!labelForm.policy_url.trim() || !labelForm.policy_ref.trim()) {
      setLabelError('Policy URL and Policy Ref are required.');
      return;
    }
    if (Number.isNaN(hours) || hours <= 0) {
      setLabelError('Expires in hours must be positive.');
      return;
    }
    const confidence =
      labelForm.confidence.trim() === ''
        ? null
        : Number(labelForm.confidence);
    if (confidence !== null && (Number.isNaN(confidence) || confidence < 0 || confidence > 1)) {
      setLabelError('Confidence must be between 0 and 1.');
      return;
    }
    const exp = Math.floor(Date.now() / 1000) + Math.floor(hours * 3600);

    manualLabelMutation.mutate({
      target: labelForm.target.trim(),
      label: labelForm.label.trim(),
      confidence,
      exp,
      policy_url: labelForm.policy_url.trim(),
      policy_ref: labelForm.policy_ref.trim(),
      topic_id: labelForm.topic_id.trim() || null
    });
  };

  const resetRuleForm = () => {
    setMode('create');
    setEditingRuleId(null);
    setRuleForm({
      name: '',
      description: '',
      priority: '0',
      is_enabled: true,
      conditions: defaultConditions,
      action: defaultAction
    });
  };

  const startEdit = (rule: ModerationRule) => {
    setMode('edit');
    setEditingRuleId(rule.rule_id);
    setRuleForm({
      name: rule.name,
      description: rule.description ?? '',
      priority: String(rule.priority),
      is_enabled: rule.is_enabled,
      conditions: formatJson(rule.conditions),
      action: formatJson(rule.action)
    });
  };

  const summary = useMemo(() => {
    const enabled = (rulesQuery.data ?? []).filter((rule) => rule.is_enabled).length;
    return { total: (rulesQuery.data ?? []).length, enabled };
  }, [rulesQuery.data]);

  return (
    <>
      <div className="hero">
        <div>
          <h1>Moderation</h1>
          <p>Rule-based labels, reports, and manual overrides.</p>
        </div>
        <button
          className="button"
          onClick={() => {
            void rulesQuery.refetch();
            void reportsQuery.refetch();
            void labelsQuery.refetch();
          }}
        >
          Refresh
        </button>
      </div>

      <div className="grid">
        <div className="card">
          <h3>{mode === 'edit' ? 'Edit Rule' : 'Create Rule'}</h3>
          <div className="field">
            <label>Name</label>
            <input
              value={ruleForm.name}
              onChange={(event) => setRuleForm((prev) => ({ ...prev, name: event.target.value }))}
            />
          </div>
          <div className="field">
            <label>Description</label>
            <input
              value={ruleForm.description}
              onChange={(event) =>
                setRuleForm((prev) => ({ ...prev, description: event.target.value }))
              }
            />
          </div>
          <div className="field">
            <label>Priority</label>
            <input
              value={ruleForm.priority}
              onChange={(event) =>
                setRuleForm((prev) => ({ ...prev, priority: event.target.value }))
              }
            />
          </div>
          <div className="field">
            <label>Enabled</label>
            <select
              value={ruleForm.is_enabled ? 'true' : 'false'}
              onChange={(event) =>
                setRuleForm((prev) => ({ ...prev, is_enabled: event.target.value === 'true' }))
              }
            >
              <option value="true">true</option>
              <option value="false">false</option>
            </select>
          </div>
          <div className="field">
            <label>Conditions (JSON)</label>
            <textarea
              rows={6}
              value={ruleForm.conditions}
              onChange={(event) =>
                setRuleForm((prev) => ({ ...prev, conditions: event.target.value }))
              }
            />
          </div>
          <div className="field">
            <label>Action (JSON)</label>
            <textarea
              rows={6}
              value={ruleForm.action}
              onChange={(event) =>
                setRuleForm((prev) => ({ ...prev, action: event.target.value }))
              }
            />
          </div>
          {ruleError && <div className="notice">{ruleError}</div>}
          <div className="row">
            <button className="button" onClick={handleRuleSubmit} disabled={saveRuleMutation.isPending}>
              {saveRuleMutation.isPending ? 'Saving...' : mode === 'edit' ? 'Update' : 'Create'}
            </button>
            {mode === 'edit' && (
              <button className="button secondary" onClick={resetRuleForm}>
                Reset
              </button>
            )}
          </div>
        </div>

        <div className="card">
          <div className="row">
            <h3>Rules</h3>
            <div className="muted">
              {summary.enabled}/{summary.total} enabled
            </div>
          </div>
          {rulesQuery.isLoading && <div className="notice">Loading rules...</div>}
          {rulesQuery.error && <div className="notice">{errorToMessage(rulesQuery.error)}</div>}
          <div className="stack">
            {(rulesQuery.data ?? []).map((rule) => (
              <div key={rule.rule_id} className="card sub-card">
                <div className="row">
                  <div>
                    <strong>{rule.name}</strong>
                    <div className="muted">Priority {rule.priority}</div>
                  </div>
                  <StatusBadge status={rule.is_enabled ? 'healthy' : 'inactive'} label={rule.is_enabled ? 'Enabled' : 'Disabled'} />
                </div>
                {rule.description && <div className="muted">{rule.description}</div>}
                <div className="muted">
                  Updated {formatTimestamp(rule.updated_at)} by {rule.updated_by}
                </div>
                <div className="row">
                  <button className="button secondary" onClick={() => startEdit(rule)}>
                    Edit
                  </button>
                  <button
                    className="button secondary"
                    onClick={() => deleteRuleMutation.mutate(rule.rule_id)}
                    disabled={deleteRuleMutation.isPending}
                  >
                    Delete
                  </button>
                </div>
              </div>
            ))}
            {(rulesQuery.data ?? []).length === 0 && (
              <div className="notice">No moderation rules configured.</div>
            )}
          </div>
        </div>
      </div>

      <div className="grid">
        <div className="card">
          <h3>Manual Label</h3>
          <div className="field">
            <label>Target (event:&lt;id&gt; or pubkey:&lt;hex&gt;)</label>
            <input
              value={labelForm.target}
              onChange={(event) => setLabelForm((prev) => ({ ...prev, target: event.target.value }))}
            />
          </div>
          <div className="field">
            <label>Label</label>
            <input
              value={labelForm.label}
              onChange={(event) => setLabelForm((prev) => ({ ...prev, label: event.target.value }))}
            />
          </div>
          <div className="field">
            <label>Confidence (0-1)</label>
            <input
              value={labelForm.confidence}
              onChange={(event) =>
                setLabelForm((prev) => ({ ...prev, confidence: event.target.value }))
              }
            />
          </div>
          <div className="field">
            <label>Expires in hours</label>
            <input
              value={labelForm.expiresInHours}
              onChange={(event) =>
                setLabelForm((prev) => ({ ...prev, expiresInHours: event.target.value }))
              }
            />
          </div>
          <div className="field">
            <label>Policy URL</label>
            <input
              value={labelForm.policy_url}
              onChange={(event) =>
                setLabelForm((prev) => ({ ...prev, policy_url: event.target.value }))
              }
            />
          </div>
          <div className="field">
            <label>Policy Ref</label>
            <input
              value={labelForm.policy_ref}
              onChange={(event) =>
                setLabelForm((prev) => ({ ...prev, policy_ref: event.target.value }))
              }
            />
          </div>
          <div className="field">
            <label>Topic (optional)</label>
            <input
              value={labelForm.topic_id}
              onChange={(event) =>
                setLabelForm((prev) => ({ ...prev, topic_id: event.target.value }))
              }
            />
          </div>
          {labelError && <div className="notice">{labelError}</div>}
          <button
            className="button"
            onClick={handleLabelSubmit}
            disabled={manualLabelMutation.isPending}
          >
            {manualLabelMutation.isPending ? 'Publishing...' : 'Publish label'}
          </button>
        </div>

        <div className="card">
          <div className="row">
            <h3>Recent Labels</h3>
            <div className="field">
              <label>Target filter</label>
              <input
                value={labelTarget}
                onChange={(event) => setLabelTarget(event.target.value)}
                placeholder="event:<id> or pubkey:<hex>"
              />
            </div>
          </div>
          {labelsQuery.isLoading && <div className="notice">Loading labels...</div>}
          {labelsQuery.error && <div className="notice">{errorToMessage(labelsQuery.error)}</div>}
          <div className="stack">
            {(labelsQuery.data ?? []).map((label) => (
              <div key={label.label_id} className="card sub-card">
                <div className="row">
                  <div>
                    <strong>{label.label}</strong>
                    <div className="muted">{label.target}</div>
                  </div>
                  <StatusBadge status={label.source === 'manual' ? 'active' : 'current'} label={label.source} />
                </div>
                <div className="muted">
                  Confidence {label.confidence ?? 'n/a'} | Expires {formatTimestamp(label.exp)}
                </div>
                <div className="muted">
                  Policy {label.policy_ref} | {label.policy_url}
                </div>
                <div className="muted">Issued {formatTimestamp(label.issued_at)}</div>
              </div>
            ))}
            {(labelsQuery.data ?? []).length === 0 && (
              <div className="notice">No labels found.</div>
            )}
          </div>
        </div>
      </div>

      <div className="card">
        <h3>Recent Reports</h3>
        {reportsQuery.isLoading && <div className="notice">Loading reports...</div>}
        {reportsQuery.error && <div className="notice">{errorToMessage(reportsQuery.error)}</div>}
        <div className="stack">
          {(reportsQuery.data ?? []).map((report) => (
            <div key={report.report_id} className="card sub-card">
              <strong>{report.target}</strong>
              <div className="muted">{report.reason}</div>
              <div className="muted">
                Reporter {report.reporter_pubkey} | {formatTimestamp(report.created_at)}
              </div>
            </div>
          ))}
          {(reportsQuery.data ?? []).length === 0 && (
            <div className="notice">No reports available.</div>
          )}
        </div>
      </div>
    </>
  );
};
