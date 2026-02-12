import { useEffect, useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import { api } from '../lib/api';
import { asRecord, findServiceByName } from '../lib/config';
import { errorToMessage } from '../lib/errorHandler';
import { formatJson, formatTimestamp } from '../lib/format';
import type {
  AuditLog,
  ModerationLabel,
  ModerationReport,
  ModerationRuleTestResult,
  ModerationRule,
  ServiceInfo
} from '../lib/types';
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

type ModerationLlmProvider = 'disabled' | 'openai' | 'local';

type LlmSettingsForm = {
  enabled: boolean;
  provider: ModerationLlmProvider;
  externalSendEnabled: boolean;
  sendPublic: boolean;
  sendInvite: boolean;
  sendFriend: boolean;
  sendFriendPlus: boolean;
  persistDecisions: boolean;
  persistRequestSnapshots: boolean;
  decisionRetentionDays: string;
  snapshotRetentionDays: string;
  maxRequestsPerDay: string;
  maxCostPerDay: string;
  maxConcurrency: string;
  truncateChars: string;
  maskPii: boolean;
};

const asBoolean = (value: unknown, fallback: boolean): boolean =>
  typeof value === 'boolean' ? value : fallback;

const asFiniteNumber = (value: unknown, fallback: number): number => {
  if (typeof value === 'number' && Number.isFinite(value)) {
    return value;
  }
  return fallback;
};

const asProvider = (value: unknown): ModerationLlmProvider => {
  if (value === 'openai' || value === 'local' || value === 'disabled') {
    return value;
  }
  return 'disabled';
};

const defaultLlmSettingsForm = (): LlmSettingsForm => ({
  enabled: false,
  provider: 'disabled',
  externalSendEnabled: false,
  sendPublic: true,
  sendInvite: false,
  sendFriend: false,
  sendFriendPlus: false,
  persistDecisions: true,
  persistRequestSnapshots: false,
  decisionRetentionDays: '90',
  snapshotRetentionDays: '7',
  maxRequestsPerDay: '0',
  maxCostPerDay: '0',
  maxConcurrency: '1',
  truncateChars: '2000',
  maskPii: true
});

const buildLlmSettingsForm = (configJson: unknown): LlmSettingsForm => {
  const defaults = defaultLlmSettingsForm();
  const moderationConfig = asRecord(configJson);
  const llm = asRecord(moderationConfig?.llm);
  const sendScope = asRecord(llm?.send_scope);
  const storage = asRecord(llm?.storage);
  const retention = asRecord(llm?.retention);

  const maxRequestsPerDay = Math.max(0, asFiniteNumber(llm?.max_requests_per_day, 0));
  const maxCostPerDay = Math.max(0, asFiniteNumber(llm?.max_cost_per_day, 0));
  const maxConcurrency = Math.max(1, asFiniteNumber(llm?.max_concurrency, 1));
  const truncateChars = Math.max(1, asFiniteNumber(llm?.truncate_chars, 2000));
  const decisionDays = Math.max(0, asFiniteNumber(retention?.decision_days, 90));
  const snapshotDays = Math.max(0, asFiniteNumber(retention?.snapshot_days, 7));

  return {
    enabled: asBoolean(llm?.enabled, defaults.enabled),
    provider: asProvider(llm?.provider),
    externalSendEnabled: asBoolean(llm?.external_send_enabled, defaults.externalSendEnabled),
    sendPublic: asBoolean(sendScope?.public, defaults.sendPublic),
    sendInvite: asBoolean(sendScope?.invite, defaults.sendInvite),
    sendFriend: asBoolean(sendScope?.friend, defaults.sendFriend),
    sendFriendPlus: asBoolean(sendScope?.friend_plus, defaults.sendFriendPlus),
    persistDecisions: asBoolean(storage?.persist_decisions, defaults.persistDecisions),
    persistRequestSnapshots: asBoolean(
      storage?.persist_request_snapshots,
      defaults.persistRequestSnapshots
    ),
    decisionRetentionDays: String(decisionDays),
    snapshotRetentionDays: String(snapshotDays),
    maxRequestsPerDay: String(maxRequestsPerDay),
    maxCostPerDay: String(maxCostPerDay),
    maxConcurrency: String(maxConcurrency),
    truncateChars: String(truncateChars),
    maskPii: asBoolean(llm?.mask_pii, defaults.maskPii)
  };
};

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
  const [ruleTestForm, setRuleTestForm] = useState({
    event_id: '',
    pubkey: '',
    kind: '1',
    content: '',
    tags: JSON.stringify([['t', 'kukuri:topic:example']], null, 2)
  });
  const [ruleTestError, setRuleTestError] = useState<string | null>(null);
  const [ruleTestResult, setRuleTestResult] = useState<ModerationRuleTestResult | null>(null);

  const [labelError, setLabelError] = useState<string | null>(null);
  const [labelTarget, setLabelTarget] = useState('');
  const [labelReviewFilter, setLabelReviewFilter] = useState<'all' | 'active' | 'disabled'>(
    'all'
  );
  const [labelActionNotes, setLabelActionNotes] = useState<Record<string, string>>({});
  const [labelForm, setLabelForm] = useState({
    target: '',
    label: '',
    confidence: '',
    expiresInHours: '24',
    policy_url: '',
    policy_ref: '',
    topic_id: ''
  });
  const [llmForm, setLlmForm] = useState<LlmSettingsForm>(defaultLlmSettingsForm());
  const [llmMessage, setLlmMessage] = useState<string | null>(null);

  const servicesQuery = useQuery<ServiceInfo[]>({
    queryKey: ['services'],
    queryFn: api.services
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
    queryKey: ['moderation-labels', labelTarget, labelReviewFilter],
    queryFn: () =>
      api.moderationLabels({
        limit: 50,
        target: labelTarget.trim() !== '' ? labelTarget.trim() : undefined,
        review_status: labelReviewFilter === 'all' ? undefined : labelReviewFilter
      })
  });

  const llmAuditQuery = useQuery<AuditLog[]>({
    queryKey: ['auditLogs', 'moderation-llm'],
    queryFn: () =>
      api.auditLogs({
        action: 'service_config.update',
        target: 'service:moderation',
        limit: 50
      })
  });

  const moderationService = useMemo(
    () => findServiceByName(servicesQuery.data, 'moderation'),
    [servicesQuery.data]
  );

  useEffect(() => {
    if (moderationService) {
      setLlmForm(buildLlmSettingsForm(moderationService.config_json));
    } else {
      setLlmForm(defaultLlmSettingsForm());
    }
  }, [moderationService?.version, moderationService?.config_json]);

  const saveLlmMutation = useMutation({
    mutationFn: (payload: unknown) =>
      api.updateServiceConfig('moderation', payload, moderationService?.version),
    onSuccess: () => {
      setLlmMessage('LLM settings saved.');
      queryClient.invalidateQueries({ queryKey: ['services'] });
      queryClient.invalidateQueries({ queryKey: ['auditLogs'] });
    },
    onError: (error) => setLlmMessage(errorToMessage(error))
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

  const testRuleMutation = useMutation({
    mutationFn: (payload: {
      conditions: unknown;
      action: unknown;
      sample: {
        event_id?: string | null;
        pubkey: string;
        kind: number;
        content: string;
        tags: string[][];
      };
    }) => api.testModerationRule(payload),
    onSuccess: (result) => {
      setRuleTestResult(result);
      setRuleTestError(null);
    },
    onError: (err) => setRuleTestError(errorToMessage(err))
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

  const reviewLabelMutation = useMutation({
    mutationFn: (payload: { labelId: string; enabled: boolean; reason?: string | null }) =>
      api.reviewModerationLabel(payload.labelId, {
        enabled: payload.enabled,
        reason: payload.reason ?? null
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['moderation-labels'] });
      queryClient.invalidateQueries({ queryKey: ['auditLogs'] });
      setLabelError(null);
    },
    onError: (err) => setLabelError(errorToMessage(err))
  });

  const rejudgeLabelMutation = useMutation({
    mutationFn: (payload: { labelId: string; reason?: string | null }) =>
      api.rejudgeModerationLabel(payload.labelId, {
        reason: payload.reason ?? null
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['moderation-labels'] });
      queryClient.invalidateQueries({ queryKey: ['auditLogs'] });
      setLabelError(null);
    },
    onError: (err) => setLabelError(errorToMessage(err))
  });

  const handleLlmSave = () => {
    setLlmMessage(null);
    if (!moderationService) {
      setLlmMessage('Moderation service config is unavailable.');
      return;
    }

    const maxRequestsPerDay = Number(llmForm.maxRequestsPerDay);
    const maxCostPerDay = Number(llmForm.maxCostPerDay);
    const maxConcurrency = Number(llmForm.maxConcurrency);
    const truncateChars = Number(llmForm.truncateChars);
    const decisionRetentionDays = Number(llmForm.decisionRetentionDays);
    const snapshotRetentionDays = Number(llmForm.snapshotRetentionDays);

    if (Number.isNaN(maxRequestsPerDay) || maxRequestsPerDay < 0) {
      setLlmMessage('Max requests/day must be 0 or greater.');
      return;
    }
    if (Number.isNaN(maxCostPerDay) || maxCostPerDay < 0) {
      setLlmMessage('Max cost/day must be 0 or greater.');
      return;
    }
    if (Number.isNaN(maxConcurrency) || maxConcurrency < 1) {
      setLlmMessage('Max concurrency must be 1 or greater.');
      return;
    }
    if (Number.isNaN(truncateChars) || truncateChars < 1) {
      setLlmMessage('Truncate chars must be 1 or greater.');
      return;
    }
    if (Number.isNaN(decisionRetentionDays) || decisionRetentionDays < 0) {
      setLlmMessage('Decision retention days must be 0 or greater.');
      return;
    }
    if (Number.isNaN(snapshotRetentionDays) || snapshotRetentionDays < 0) {
      setLlmMessage('Snapshot retention days must be 0 or greater.');
      return;
    }

    const currentConfig = asRecord(moderationService.config_json) ?? {};
    const currentLlm = asRecord(currentConfig.llm) ?? {};
    const currentSendScope = asRecord(currentLlm.send_scope) ?? {};
    const currentStorage = asRecord(currentLlm.storage) ?? {};
    const currentRetention = asRecord(currentLlm.retention) ?? {};

    saveLlmMutation.mutate({
      ...currentConfig,
      llm: {
        ...currentLlm,
        enabled: llmForm.enabled,
        provider: llmForm.provider,
        external_send_enabled: llmForm.provider === 'openai' ? llmForm.externalSendEnabled : false,
        send_scope: {
          ...currentSendScope,
          public: llmForm.sendPublic,
          invite: llmForm.sendInvite,
          friend: llmForm.sendFriend,
          friend_plus: llmForm.sendFriendPlus
        },
        storage: {
          ...currentStorage,
          persist_decisions: llmForm.persistDecisions,
          persist_request_snapshots: llmForm.persistRequestSnapshots
        },
        retention: {
          ...currentRetention,
          decision_days: Math.floor(decisionRetentionDays),
          snapshot_days: Math.floor(snapshotRetentionDays)
        },
        truncate_chars: Math.floor(truncateChars),
        mask_pii: llmForm.maskPii,
        max_requests_per_day: Math.floor(maxRequestsPerDay),
        max_cost_per_day: maxCostPerDay,
        max_concurrency: Math.floor(maxConcurrency)
      }
    });
  };

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

  const handleRuleTest = () => {
    setRuleTestError(null);
    setRuleTestResult(null);

    let conditions: unknown;
    let action: unknown;
    try {
      conditions = JSON.parse(ruleForm.conditions);
      action = JSON.parse(ruleForm.action);
    } catch (err) {
      setRuleTestError(errorToMessage(err));
      return;
    }

    const pubkey = ruleTestForm.pubkey.trim();
    if (pubkey.length !== 64 || !/^[0-9a-f]+$/i.test(pubkey)) {
      setRuleTestError('Sample pubkey must be a 64-char hex string.');
      return;
    }
    const kind = Number(ruleTestForm.kind);
    if (Number.isNaN(kind) || kind < 0) {
      setRuleTestError('Sample kind must be 0 or greater.');
      return;
    }
    let tags: string[][];
    try {
      const parsed = JSON.parse(ruleTestForm.tags);
      if (!Array.isArray(parsed) || parsed.some((tag) => !Array.isArray(tag))) {
        setRuleTestError('Tags must be a JSON array of string arrays.');
        return;
      }
      tags = parsed.map((tag) =>
        tag.map((entry) => (typeof entry === 'string' ? entry : String(entry)))
      );
    } catch (err) {
      setRuleTestError(errorToMessage(err));
      return;
    }

    testRuleMutation.mutate({
      conditions,
      action,
      sample: {
        event_id:
          ruleTestForm.event_id.trim() === '' ? null : ruleTestForm.event_id.trim(),
        pubkey,
        kind,
        content: ruleTestForm.content,
        tags
      }
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

  const labelReviewStatus = (label: ModerationLabel): 'active' | 'disabled' =>
    label.review_status === 'disabled' ? 'disabled' : 'active';

  const labelActionNote = (labelId: string): string => labelActionNotes[labelId] ?? '';

  const setLabelActionNote = (labelId: string, note: string) => {
    setLabelActionNotes((previous) => ({ ...previous, [labelId]: note }));
  };

  const handleReviewAction = (label: ModerationLabel, enabled: boolean) => {
    setLabelError(null);
    const note = labelActionNote(label.label_id).trim();
    if (!enabled && note === '') {
      setLabelError('Reason is required when disabling a label.');
      return;
    }

    reviewLabelMutation.mutate({
      labelId: label.label_id,
      enabled,
      reason: note === '' ? null : note
    });
  };

  const handleRejudgeAction = (label: ModerationLabel) => {
    setLabelError(null);
    const note = labelActionNote(label.label_id).trim();
    rejudgeLabelMutation.mutate({
      labelId: label.label_id,
      reason: note === '' ? null : note
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
            void servicesQuery.refetch();
            void llmAuditQuery.refetch();
            void rulesQuery.refetch();
            void reportsQuery.refetch();
            void labelsQuery.refetch();
          }}
        >
          Refresh
        </button>
      </div>

      <div className="card">
        <div className="row">
          <div>
            <h3>LLM Integration Settings</h3>
            <p>Configure provider, sending scope, storage/retention, and runtime budgets.</p>
          </div>
          {moderationService && (
            <StatusBadge status={moderationService.health?.status ?? 'unknown'} />
          )}
        </div>
        {!moderationService && (
          <div className="notice">Moderation service config is unavailable.</div>
        )}
        {moderationService && (
          <>
            <div className="muted">
              Version {moderationService.version} | Updated{' '}
              {formatTimestamp(moderationService.updated_at)} by {moderationService.updated_by}
            </div>

            <div className="grid">
              <div className="card sub-card">
                <h3>Provider</h3>
                <div className="field">
                  <label htmlFor="llm-enabled">LLM enabled</label>
                  <select
                    id="llm-enabled"
                    value={llmForm.enabled ? 'true' : 'false'}
                    onChange={(event) =>
                      setLlmForm((prev) => ({ ...prev, enabled: event.target.value === 'true' }))
                    }
                  >
                    <option value="true">true</option>
                    <option value="false">false</option>
                  </select>
                </div>
                <div className="field">
                  <label htmlFor="llm-provider">Provider</label>
                  <select
                    id="llm-provider"
                    value={llmForm.provider}
                    onChange={(event) =>
                      setLlmForm((prev) => ({
                        ...prev,
                        provider: event.target.value as ModerationLlmProvider
                      }))
                    }
                  >
                    <option value="disabled">disabled</option>
                    <option value="openai">openai</option>
                    <option value="local">local</option>
                  </select>
                </div>
                <div className="field">
                  <label htmlFor="llm-external-send">External send (OpenAI only)</label>
                  <select
                    id="llm-external-send"
                    value={llmForm.externalSendEnabled ? 'true' : 'false'}
                    onChange={(event) =>
                      setLlmForm((prev) => ({
                        ...prev,
                        externalSendEnabled: event.target.value === 'true'
                      }))
                    }
                    disabled={llmForm.provider !== 'openai'}
                  >
                    <option value="false">false</option>
                    <option value="true">true</option>
                  </select>
                </div>
              </div>

              <div className="card sub-card">
                <h3>Send Scope</h3>
                <div className="field">
                  <label htmlFor="llm-scope-public">
                    <input
                      id="llm-scope-public"
                      type="checkbox"
                      checked={llmForm.sendPublic}
                      onChange={(event) =>
                        setLlmForm((prev) => ({ ...prev, sendPublic: event.target.checked }))
                      }
                    />
                    {' '}public
                  </label>
                </div>
                <div className="field">
                  <label htmlFor="llm-scope-invite">
                    <input
                      id="llm-scope-invite"
                      type="checkbox"
                      checked={llmForm.sendInvite}
                      onChange={(event) =>
                        setLlmForm((prev) => ({ ...prev, sendInvite: event.target.checked }))
                      }
                    />
                    {' '}invite
                  </label>
                </div>
                <div className="field">
                  <label htmlFor="llm-scope-friend">
                    <input
                      id="llm-scope-friend"
                      type="checkbox"
                      checked={llmForm.sendFriend}
                      onChange={(event) =>
                        setLlmForm((prev) => ({ ...prev, sendFriend: event.target.checked }))
                      }
                    />
                    {' '}friend
                  </label>
                </div>
                <div className="field">
                  <label htmlFor="llm-scope-friend-plus">
                    <input
                      id="llm-scope-friend-plus"
                      type="checkbox"
                      checked={llmForm.sendFriendPlus}
                      onChange={(event) =>
                        setLlmForm((prev) => ({ ...prev, sendFriendPlus: event.target.checked }))
                      }
                    />
                    {' '}friend_plus
                  </label>
                </div>
              </div>

              <div className="card sub-card">
                <h3>Storage / Retention</h3>
                <div className="field">
                  <label htmlFor="llm-persist-decisions">
                    <input
                      id="llm-persist-decisions"
                      type="checkbox"
                      checked={llmForm.persistDecisions}
                      onChange={(event) =>
                        setLlmForm((prev) => ({ ...prev, persistDecisions: event.target.checked }))
                      }
                    />
                    {' '}Persist decisions
                  </label>
                </div>
                <div className="field">
                  <label htmlFor="llm-persist-snapshots">
                    <input
                      id="llm-persist-snapshots"
                      type="checkbox"
                      checked={llmForm.persistRequestSnapshots}
                      onChange={(event) =>
                        setLlmForm((prev) => ({
                          ...prev,
                          persistRequestSnapshots: event.target.checked
                        }))
                      }
                    />
                    {' '}Persist request snapshots
                  </label>
                </div>
                <div className="field">
                  <label htmlFor="llm-decision-retention">Decision retention days</label>
                  <input
                    id="llm-decision-retention"
                    type="number"
                    min={0}
                    value={llmForm.decisionRetentionDays}
                    onChange={(event) =>
                      setLlmForm((prev) => ({
                        ...prev,
                        decisionRetentionDays: event.target.value
                      }))
                    }
                  />
                </div>
                <div className="field">
                  <label htmlFor="llm-snapshot-retention">Snapshot retention days</label>
                  <input
                    id="llm-snapshot-retention"
                    type="number"
                    min={0}
                    value={llmForm.snapshotRetentionDays}
                    onChange={(event) =>
                      setLlmForm((prev) => ({
                        ...prev,
                        snapshotRetentionDays: event.target.value
                      }))
                    }
                  />
                </div>
              </div>

              <div className="card sub-card">
                <h3>Budget / Runtime</h3>
                <div className="field">
                  <label htmlFor="llm-max-requests">Max requests per day</label>
                  <input
                    id="llm-max-requests"
                    type="number"
                    min={0}
                    value={llmForm.maxRequestsPerDay}
                    onChange={(event) =>
                      setLlmForm((prev) => ({ ...prev, maxRequestsPerDay: event.target.value }))
                    }
                  />
                </div>
                <div className="field">
                  <label htmlFor="llm-max-cost">Max cost per day</label>
                  <input
                    id="llm-max-cost"
                    type="number"
                    min={0}
                    step="0.0001"
                    value={llmForm.maxCostPerDay}
                    onChange={(event) =>
                      setLlmForm((prev) => ({ ...prev, maxCostPerDay: event.target.value }))
                    }
                  />
                </div>
                <div className="field">
                  <label htmlFor="llm-max-concurrency">Max concurrency</label>
                  <input
                    id="llm-max-concurrency"
                    type="number"
                    min={1}
                    value={llmForm.maxConcurrency}
                    onChange={(event) =>
                      setLlmForm((prev) => ({ ...prev, maxConcurrency: event.target.value }))
                    }
                  />
                </div>
                <div className="field">
                  <label htmlFor="llm-truncate-chars">Truncate chars</label>
                  <input
                    id="llm-truncate-chars"
                    type="number"
                    min={1}
                    value={llmForm.truncateChars}
                    onChange={(event) =>
                      setLlmForm((prev) => ({ ...prev, truncateChars: event.target.value }))
                    }
                  />
                </div>
                <div className="field">
                  <label htmlFor="llm-mask-pii">
                    <input
                      id="llm-mask-pii"
                      type="checkbox"
                      checked={llmForm.maskPii}
                      onChange={(event) =>
                        setLlmForm((prev) => ({ ...prev, maskPii: event.target.checked }))
                      }
                    />
                    {' '}Mask PII before send
                  </label>
                </div>
              </div>
            </div>

            {llmMessage && <div className="notice">{llmMessage}</div>}
            <button className="button" onClick={handleLlmSave} disabled={saveLlmMutation.isPending}>
              {saveLlmMutation.isPending ? 'Saving...' : 'Save LLM settings'}
            </button>
          </>
        )}
      </div>

      <div className="card">
        <h3>Recent LLM Config Audits</h3>
        {llmAuditQuery.isLoading && <div className="notice">Loading audit logs...</div>}
        {llmAuditQuery.error && <div className="notice">{errorToMessage(llmAuditQuery.error)}</div>}
        <table className="table">
          <thead>
            <tr>
              <th>Time</th>
              <th>Action</th>
              <th>Target</th>
              <th>Actor</th>
            </tr>
          </thead>
          <tbody>
            {(llmAuditQuery.data ?? []).map((log) => (
              <tr key={log.audit_id}>
                <td>{formatTimestamp(log.created_at)}</td>
                <td>{log.action}</td>
                <td>{log.target}</td>
                <td>{log.actor_admin_user_id}</td>
              </tr>
            ))}
            {(llmAuditQuery.data ?? []).length === 0 && (
              <tr>
                <td colSpan={4}>No moderation config audit logs found.</td>
              </tr>
            )}
          </tbody>
        </table>
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

      <div className="card">
        <h3>Rule Test Runner</h3>
        <p>Run current rule JSON against a sample event without storing labels.</p>
        <div className="field">
          <label htmlFor="rule-test-event-id">Sample event id (optional)</label>
          <input
            id="rule-test-event-id"
            value={ruleTestForm.event_id}
            onChange={(event) =>
              setRuleTestForm((prev) => ({ ...prev, event_id: event.target.value }))
            }
            placeholder="event-123"
          />
        </div>
        <div className="field">
          <label htmlFor="rule-test-pubkey">Sample pubkey</label>
          <input
            id="rule-test-pubkey"
            value={ruleTestForm.pubkey}
            onChange={(event) =>
              setRuleTestForm((prev) => ({ ...prev, pubkey: event.target.value }))
            }
            placeholder="64-char hex pubkey"
          />
        </div>
        <div className="field">
          <label htmlFor="rule-test-kind">Sample kind</label>
          <input
            id="rule-test-kind"
            type="number"
            min={0}
            value={ruleTestForm.kind}
            onChange={(event) =>
              setRuleTestForm((prev) => ({ ...prev, kind: event.target.value }))
            }
          />
        </div>
        <div className="field">
          <label htmlFor="rule-test-content">Sample content</label>
          <textarea
            id="rule-test-content"
            rows={4}
            value={ruleTestForm.content}
            onChange={(event) =>
              setRuleTestForm((prev) => ({ ...prev, content: event.target.value }))
            }
          />
        </div>
        <div className="field">
          <label htmlFor="rule-test-tags">Sample tags (JSON)</label>
          <textarea
            id="rule-test-tags"
            rows={4}
            value={ruleTestForm.tags}
            onChange={(event) =>
              setRuleTestForm((prev) => ({ ...prev, tags: event.target.value }))
            }
          />
        </div>
        {ruleTestError && <div className="notice">{ruleTestError}</div>}
        <button className="button" onClick={handleRuleTest} disabled={testRuleMutation.isPending}>
          {testRuleMutation.isPending ? 'Testing...' : 'Run rule test'}
        </button>
        {ruleTestResult && (
          <div className="card sub-card">
            <div className="row">
              <strong>{ruleTestResult.matched ? 'Matched' : 'Not matched'}</strong>
              <StatusBadge
                status={ruleTestResult.matched ? 'healthy' : 'inactive'}
                label={ruleTestResult.matched ? 'match' : 'no-match'}
              />
            </div>
            <div className="muted">{ruleTestResult.reasons.join(' | ')}</div>
            {ruleTestResult.preview && (
              <div className="muted">
                Preview: {ruleTestResult.preview.label} ({ruleTestResult.preview.target})
              </div>
            )}
          </div>
        )}
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
            <div className="row">
              <div className="field">
                <label>Target filter</label>
                <input
                  value={labelTarget}
                  onChange={(event) => setLabelTarget(event.target.value)}
                  placeholder="event:<id> or pubkey:<hex>"
                />
              </div>
              <div className="field">
                <label>Review status</label>
                <select
                  value={labelReviewFilter}
                  onChange={(event) =>
                    setLabelReviewFilter(event.target.value as 'all' | 'active' | 'disabled')
                  }
                >
                  <option value="all">all</option>
                  <option value="active">active</option>
                  <option value="disabled">disabled</option>
                </select>
              </div>
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
                  <StatusBadge
                    status={labelReviewStatus(label)}
                    label={`review:${labelReviewStatus(label)}`}
                  />
                </div>
                <div className="muted">
                  Confidence {label.confidence ?? 'n/a'} | Expires {formatTimestamp(label.exp)}
                </div>
                <div className="muted">
                  Policy {label.policy_ref} | {label.policy_url}
                </div>
                <div className="muted">Source {label.source}</div>
                <div className="muted">
                  Review by {label.reviewed_by ?? 'n/a'} |{' '}
                  {label.reviewed_at ? formatTimestamp(label.reviewed_at) : 'n/a'}
                </div>
                <div className="muted">Review reason: {label.review_reason ?? 'n/a'}</div>
                <div className="muted">Issued {formatTimestamp(label.issued_at)}</div>
                <div className="field">
                  <label htmlFor={`label-action-note-${label.label_id}`}>Operator note</label>
                  <input
                    id={`label-action-note-${label.label_id}`}
                    value={labelActionNote(label.label_id)}
                    onChange={(event) => setLabelActionNote(label.label_id, event.target.value)}
                    placeholder="reason for disable/rejudge"
                  />
                </div>
                <div className="row">
                  {labelReviewStatus(label) === 'active' ? (
                    <button
                      className="button secondary"
                      onClick={() => handleReviewAction(label, false)}
                      disabled={reviewLabelMutation.isPending}
                    >
                      {reviewLabelMutation.isPending ? 'Updating...' : 'Disable label'}
                    </button>
                  ) : (
                    <button
                      className="button secondary"
                      onClick={() => handleReviewAction(label, true)}
                      disabled={reviewLabelMutation.isPending}
                    >
                      {reviewLabelMutation.isPending ? 'Updating...' : 'Enable label'}
                    </button>
                  )}
                  <button
                    className="button secondary"
                    onClick={() => handleRejudgeAction(label)}
                    disabled={rejudgeLabelMutation.isPending}
                  >
                    {rejudgeLabelMutation.isPending ? 'Queueing...' : 'Trigger rejudge'}
                  </button>
                </div>
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
