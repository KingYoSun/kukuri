import { useEffect, useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import { StatusBadge } from '../components/StatusBadge';
import { api } from '../lib/api';
import { asRecord, findServiceByName } from '../lib/config';
import { errorToMessage } from '../lib/errorHandler';
import { formatJson, formatTimestamp } from '../lib/format';
import type { AuditLog, DsarJob, DsarJobType, Policy, ServiceInfo } from '../lib/types';

const policyBadge = (policy: Policy | null) => {
  if (!policy) {
    return { status: 'inactive', label: 'Not configured' };
  }
  if (policy.is_current) {
    return { status: 'current', label: 'Current' };
  }
  if (policy.published_at) {
    return { status: 'active', label: 'Published' };
  }
  return { status: 'inactive', label: 'Draft' };
};

const asNumber = (value: unknown): number | null => {
  if (typeof value === 'number') {
    return value;
  }
  return null;
};

const isDsarJobType = (value: string): value is DsarJobType =>
  value === 'export' || value === 'deletion';

const dsarTypeLabel = (value: DsarJobType) => (value === 'export' ? 'Export' : 'Deletion');

type ConfigEditorProps = {
  title: string;
  subtitle: string;
  service: ServiceInfo | null;
  draft: string;
  onChange: (value: string) => void;
  onSave: () => void;
  isSaving: boolean;
  message: string | null;
};

const ConfigEditor = ({
  title,
  subtitle,
  service,
  draft,
  onChange,
  onSave,
  isSaving,
  message
}: ConfigEditorProps) => {
  if (!service) {
    return (
      <div className="card">
        <h3>{title}</h3>
        <p>{subtitle}</p>
        <div className="notice">Service config is not available.</div>
      </div>
    );
  }

  const inputId = `${service.service}-config-json`;

  return (
    <div className="card">
      <div className="row">
        <div>
          <h3>{title}</h3>
          <p>{subtitle}</p>
        </div>
        <StatusBadge status={service.health?.status ?? 'unknown'} />
      </div>
      <div className="muted">
        Version {service.version} | Updated {formatTimestamp(service.updated_at)} by{' '}
        {service.updated_by}
      </div>
      <div className="field">
        <label htmlFor={inputId}>Config JSON</label>
        <textarea
          id={inputId}
          rows={12}
          value={draft}
          onChange={(event) => onChange(event.target.value)}
        />
      </div>
      {message && <div className="notice">{message}</div>}
      <button className="button" onClick={onSave} disabled={isSaving}>
        {isSaving ? 'Saving...' : 'Save config'}
      </button>
    </div>
  );
};

export const PrivacyDataPage = () => {
  const queryClient = useQueryClient();
  const [relayDraft, setRelayDraft] = useState('{}');
  const [userApiDraft, setUserApiDraft] = useState('{}');
  const [relayMessage, setRelayMessage] = useState<string | null>(null);
  const [userApiMessage, setUserApiMessage] = useState<string | null>(null);
  const [dsarMessage, setDsarMessage] = useState<string | null>(null);

  const servicesQuery = useQuery<ServiceInfo[]>({
    queryKey: ['services'],
    queryFn: api.services
  });

  const policiesQuery = useQuery<Policy[]>({
    queryKey: ['policies'],
    queryFn: api.policies
  });

  const auditQuery = useQuery<AuditLog[]>({
    queryKey: ['auditLogs', 'privacy-data'],
    queryFn: () => api.auditLogs({ limit: 200 })
  });

  const dsarJobsQuery = useQuery<DsarJob[]>({
    queryKey: ['dsar-jobs'],
    queryFn: () => api.dsarJobs({ limit: 200 })
  });

  const relayService = useMemo(
    () => findServiceByName(servicesQuery.data, 'relay'),
    [servicesQuery.data]
  );
  const userApiService = useMemo(
    () => findServiceByName(servicesQuery.data, 'user-api'),
    [servicesQuery.data]
  );

  useEffect(() => {
    if (relayService) {
      setRelayDraft(formatJson(relayService.config_json));
    }
  }, [relayService?.version, relayService?.config_json]);

  useEffect(() => {
    if (userApiService) {
      setUserApiDraft(formatJson(userApiService.config_json));
    }
  }, [userApiService?.version, userApiService?.config_json]);

  const saveRelayMutation = useMutation({
    mutationFn: (payload: unknown) =>
      api.updateServiceConfig('relay', payload, relayService?.version),
    onSuccess: () => {
      setRelayMessage('Relay config saved.');
      queryClient.invalidateQueries({ queryKey: ['services'] });
      queryClient.invalidateQueries({ queryKey: ['auditLogs'] });
    },
    onError: (error) => setRelayMessage(errorToMessage(error))
  });

  const saveUserApiMutation = useMutation({
    mutationFn: (payload: unknown) =>
      api.updateServiceConfig('user-api', payload, userApiService?.version),
    onSuccess: () => {
      setUserApiMessage('User API config saved.');
      queryClient.invalidateQueries({ queryKey: ['services'] });
      queryClient.invalidateQueries({ queryKey: ['auditLogs'] });
    },
    onError: (error) => setUserApiMessage(errorToMessage(error))
  });

  const retryDsarJobMutation = useMutation({
    mutationFn: (payload: { requestType: DsarJobType; jobId: string }) =>
      api.retryDsarJob(payload.requestType, payload.jobId),
    onSuccess: (job) => {
      setDsarMessage(`Retry queued for ${job.request_type} job ${job.job_id}.`);
      queryClient.invalidateQueries({ queryKey: ['dsar-jobs'] });
      queryClient.invalidateQueries({ queryKey: ['auditLogs'] });
    },
    onError: (error) => setDsarMessage(errorToMessage(error))
  });

  const cancelDsarJobMutation = useMutation({
    mutationFn: (payload: { requestType: DsarJobType; jobId: string }) =>
      api.cancelDsarJob(payload.requestType, payload.jobId),
    onSuccess: (job) => {
      setDsarMessage(`Job ${job.job_id} canceled.`);
      queryClient.invalidateQueries({ queryKey: ['dsar-jobs'] });
      queryClient.invalidateQueries({ queryKey: ['auditLogs'] });
    },
    onError: (error) => setDsarMessage(errorToMessage(error))
  });

  const currentTerms = useMemo(
    () => (policiesQuery.data ?? []).find((policy) => policy.policy_type === 'terms' && policy.is_current) ?? null,
    [policiesQuery.data]
  );
  const currentPrivacy = useMemo(
    () =>
      (policiesQuery.data ?? []).find((policy) => policy.policy_type === 'privacy' && policy.is_current) ??
      null,
    [policiesQuery.data]
  );

  const relayRetention = useMemo(() => {
    const relayConfig = asRecord(relayService?.config_json);
    if (!relayConfig) {
      return null;
    }
    return asRecord(relayConfig.retention);
  }, [relayService]);

  const userRateLimit = useMemo(() => {
    const userApiConfig = asRecord(userApiService?.config_json);
    if (!userApiConfig) {
      return null;
    }
    return asRecord(userApiConfig.rate_limit);
  }, [userApiService]);

  const filteredAuditLogs = useMemo(
    () =>
      (auditQuery.data ?? []).filter((log) => {
        if (log.action.startsWith('policy.')) {
          return true;
        }
        if (log.action.startsWith('dsar.job.')) {
          return true;
        }
        if (log.action !== 'service_config.update') {
          return false;
        }
        return log.target === 'service:relay' || log.target === 'service:user-api';
      }),
    [auditQuery.data]
  );

  const dsarSummary = useMemo(() => {
    const jobs = dsarJobsQuery.data ?? [];
    return {
      total: jobs.length,
      queued: jobs.filter((job) => job.status === 'queued').length,
      running: jobs.filter((job) => job.status === 'running').length,
      completed: jobs.filter((job) => job.status === 'completed').length,
      failed: jobs.filter((job) => job.status === 'failed').length
    };
  }, [dsarJobsQuery.data]);

  const handleRetryJob = (job: DsarJob) => {
    setDsarMessage(null);
    if (!isDsarJobType(job.request_type)) {
      setDsarMessage(`Unsupported request type: ${job.request_type}`);
      return;
    }
    retryDsarJobMutation.mutate({
      requestType: job.request_type,
      jobId: job.job_id
    });
  };

  const handleCancelJob = (job: DsarJob) => {
    setDsarMessage(null);
    if (!isDsarJobType(job.request_type)) {
      setDsarMessage(`Unsupported request type: ${job.request_type}`);
      return;
    }
    cancelDsarJobMutation.mutate({
      requestType: job.request_type,
      jobId: job.job_id
    });
  };

  const saveRelay = () => {
    setRelayMessage(null);
    try {
      const parsed = JSON.parse(relayDraft);
      saveRelayMutation.mutate(parsed);
    } catch {
      setRelayMessage('Relay config must be valid JSON.');
    }
  };

  const saveUserApi = () => {
    setUserApiMessage(null);
    try {
      const parsed = JSON.parse(userApiDraft);
      saveUserApiMutation.mutate(parsed);
    } catch {
      setUserApiMessage('User API config must be valid JSON.');
    }
  };

  return (
    <>
      <div className="hero">
        <div>
          <h1>Privacy / Data</h1>
          <p>Manage policy visibility and data-handling runtime settings.</p>
        </div>
        <button
          className="button"
          onClick={() => {
            void policiesQuery.refetch();
            void servicesQuery.refetch();
            void dsarJobsQuery.refetch();
            void auditQuery.refetch();
          }}
        >
          Refresh
        </button>
      </div>

      <div className="grid">
        <div className="card">
          <h3>Current Policies</h3>
          {policiesQuery.isLoading && <div className="notice">Loading policies...</div>}
          {policiesQuery.error && <div className="notice">{errorToMessage(policiesQuery.error)}</div>}
          <div className="stack">
            <div className="card sub-card">
              <div className="row">
                <div>
                  <strong>Terms of Service</strong>
                  <div className="muted">
                    {currentTerms ? `${currentTerms.version} (${currentTerms.locale})` : 'No active policy'}
                  </div>
                </div>
                <StatusBadge {...policyBadge(currentTerms)} />
              </div>
              <div className="muted">
                Effective {formatTimestamp(currentTerms?.effective_at ?? null)}
              </div>
            </div>
            <div className="card sub-card">
              <div className="row">
                <div>
                  <strong>Privacy Policy</strong>
                  <div className="muted">
                    {currentPrivacy
                      ? `${currentPrivacy.version} (${currentPrivacy.locale})`
                      : 'No active policy'}
                  </div>
                </div>
                <StatusBadge {...policyBadge(currentPrivacy)} />
              </div>
              <div className="muted">
                Effective {formatTimestamp(currentPrivacy?.effective_at ?? null)}
              </div>
            </div>
          </div>
        </div>

        <div className="card">
          <h3>Retention Snapshot</h3>
          {!relayRetention && <div className="notice">Relay retention config is unavailable.</div>}
          {relayRetention && (
            <table className="table">
              <thead>
                <tr>
                  <th>Key</th>
                  <th>Value</th>
                </tr>
              </thead>
              <tbody>
                <tr>
                  <td>events_days</td>
                  <td>{asNumber(relayRetention.events_days) ?? '—'}</td>
                </tr>
                <tr>
                  <td>outbox_days</td>
                  <td>{asNumber(relayRetention.outbox_days) ?? '—'}</td>
                </tr>
                <tr>
                  <td>dedupe_days</td>
                  <td>{asNumber(relayRetention.dedupe_days) ?? '—'}</td>
                </tr>
                <tr>
                  <td>tombstone_days</td>
                  <td>{asNumber(relayRetention.tombstone_days) ?? '—'}</td>
                </tr>
                <tr>
                  <td>cleanup_interval_seconds</td>
                  <td>{asNumber(relayRetention.cleanup_interval_seconds) ?? '—'}</td>
                </tr>
              </tbody>
            </table>
          )}
          <div className="divider" />
          <h3>User API Rate Limit</h3>
          {!userRateLimit && <div className="notice">User API rate limit config is unavailable.</div>}
          {userRateLimit && (
            <table className="table">
              <thead>
                <tr>
                  <th>Key</th>
                  <th>Value</th>
                </tr>
              </thead>
              <tbody>
                <tr>
                  <td>auth_per_minute</td>
                  <td>{asNumber(userRateLimit.auth_per_minute) ?? '—'}</td>
                </tr>
                <tr>
                  <td>public_per_minute</td>
                  <td>{asNumber(userRateLimit.public_per_minute) ?? '—'}</td>
                </tr>
                <tr>
                  <td>protected_per_minute</td>
                  <td>{asNumber(userRateLimit.protected_per_minute) ?? '—'}</td>
                </tr>
              </tbody>
            </table>
          )}
        </div>
      </div>

      <div className="card">
        <div className="row">
          <div>
            <h3>DSAR Operations</h3>
            <p className="muted">
              Total {dsarSummary.total} | Queued {dsarSummary.queued} | Running {dsarSummary.running} |
              Completed {dsarSummary.completed} | Failed {dsarSummary.failed}
            </p>
          </div>
        </div>
        {dsarMessage && <div className="notice">{dsarMessage}</div>}
        {dsarJobsQuery.isLoading && <div className="notice">Loading DSAR jobs...</div>}
        {dsarJobsQuery.error && <div className="notice">{errorToMessage(dsarJobsQuery.error)}</div>}
        <table className="table">
          <thead>
            <tr>
              <th>Type</th>
              <th>Request ID</th>
              <th>Requester</th>
              <th>Status</th>
              <th>Created</th>
              <th>Completed</th>
              <th>Error</th>
              <th>Actions</th>
            </tr>
          </thead>
          <tbody>
            {(dsarJobsQuery.data ?? []).map((job) => {
              const canRetry = job.status === 'failed' || job.status === 'completed';
              const canCancel = job.status === 'queued' || job.status === 'running';
              const retryLabel = retryDsarJobMutation.isPending ? 'Retrying...' : 'Retry';
              const cancelLabel = cancelDsarJobMutation.isPending ? 'Canceling...' : 'Cancel';
              return (
                <tr key={`${job.request_type}-${job.job_id}`}>
                  <td>{isDsarJobType(job.request_type) ? dsarTypeLabel(job.request_type) : job.request_type}</td>
                  <td>{job.job_id}</td>
                  <td>{job.requester_pubkey}</td>
                  <td>
                    <StatusBadge status={job.status} />
                  </td>
                  <td>{formatTimestamp(job.created_at)}</td>
                  <td>{formatTimestamp(job.completed_at)}</td>
                  <td>{job.error_message ?? '—'}</td>
                  <td>
                    <div className="row">
                      <button
                        className="button secondary"
                        onClick={() => handleRetryJob(job)}
                        disabled={!canRetry || retryDsarJobMutation.isPending}
                      >
                        {retryLabel}
                      </button>
                      <button
                        className="button"
                        onClick={() => handleCancelJob(job)}
                        disabled={!canCancel || cancelDsarJobMutation.isPending}
                      >
                        {cancelLabel}
                      </button>
                    </div>
                  </td>
                </tr>
              );
            })}
            {(dsarJobsQuery.data ?? []).length === 0 && (
              <tr>
                <td colSpan={8}>No DSAR jobs found.</td>
              </tr>
            )}
          </tbody>
        </table>
      </div>

      <div className="grid">
        <ConfigEditor
          title="Relay Data Policy"
          subtitle="Retention and ingest controls used by relay and downstream data flow."
          service={relayService}
          draft={relayDraft}
          onChange={setRelayDraft}
          onSave={saveRelay}
          isSaving={saveRelayMutation.isPending}
          message={relayMessage}
        />
        <ConfigEditor
          title="User API Data Policy"
          subtitle="Request throttling and public/protected path controls."
          service={userApiService}
          draft={userApiDraft}
          onChange={setUserApiDraft}
          onSave={saveUserApi}
          isSaving={saveUserApiMutation.isPending}
          message={userApiMessage}
        />
      </div>

      <div className="card">
        <h3>Recent Privacy/Data Audits</h3>
        {auditQuery.isLoading && <div className="notice">Loading audit logs...</div>}
        {auditQuery.error && <div className="notice">{errorToMessage(auditQuery.error)}</div>}
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
            {filteredAuditLogs.map((log) => (
              <tr key={log.audit_id}>
                <td>{formatTimestamp(log.created_at)}</td>
                <td>{log.action}</td>
                <td>{log.target}</td>
                <td>{log.actor_admin_user_id}</td>
              </tr>
            ))}
            {filteredAuditLogs.length === 0 && (
              <tr>
                <td colSpan={4}>No privacy/data audit logs found.</td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </>
  );
};
