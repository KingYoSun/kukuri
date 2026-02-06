import { useEffect, useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import { StatusBadge } from '../components/StatusBadge';
import { api } from '../lib/api';
import { findServiceByName } from '../lib/config';
import { errorToMessage } from '../lib/errorHandler';
import { formatJson, formatTimestamp } from '../lib/format';
import type { AuditLog, ReindexResponse, ServiceInfo } from '../lib/types';

export const IndexPage = () => {
  const queryClient = useQueryClient();
  const [indexDraft, setIndexDraft] = useState('{}');
  const [indexMessage, setIndexMessage] = useState<string | null>(null);
  const [topicId, setTopicId] = useState('');
  const [reindexMessage, setReindexMessage] = useState<string | null>(null);
  const [reindexResult, setReindexResult] = useState<ReindexResponse | null>(null);

  const servicesQuery = useQuery<ServiceInfo[]>({
    queryKey: ['services'],
    queryFn: api.services
  });

  const auditQuery = useQuery<AuditLog[]>({
    queryKey: ['auditLogs', 'index.reindex.request'],
    queryFn: () => api.auditLogs({ action: 'index.reindex.request', limit: 100 })
  });

  const indexService = useMemo(
    () => findServiceByName(servicesQuery.data, 'index'),
    [servicesQuery.data]
  );

  useEffect(() => {
    if (indexService) {
      setIndexDraft(formatJson(indexService.config_json));
    }
  }, [indexService?.version, indexService?.config_json]);

  const saveMutation = useMutation({
    mutationFn: (payload: unknown) => api.updateServiceConfig('index', payload, indexService?.version),
    onSuccess: () => {
      setIndexMessage('Index config saved.');
      queryClient.invalidateQueries({ queryKey: ['services'] });
      queryClient.invalidateQueries({ queryKey: ['auditLogs'] });
    },
    onError: (error) => setIndexMessage(errorToMessage(error))
  });

  const reindexMutation = useMutation({
    mutationFn: (topic: string) => api.reindex(topic === '' ? null : topic),
    onSuccess: (result) => {
      setReindexResult(result);
      setReindexMessage('Reindex request accepted.');
      queryClient.invalidateQueries({ queryKey: ['auditLogs'] });
    },
    onError: (error) => setReindexMessage(errorToMessage(error))
  });

  const saveIndexConfig = () => {
    setIndexMessage(null);
    try {
      const parsed = JSON.parse(indexDraft);
      saveMutation.mutate(parsed);
    } catch {
      setIndexMessage('Index config must be valid JSON.');
    }
  };

  const submitReindex = () => {
    setReindexMessage(null);
    setReindexResult(null);
    reindexMutation.mutate(topicId.trim());
  };

  return (
    <>
      <div className="hero">
        <div>
          <h1>Index</h1>
          <p>Manage index runtime settings and trigger Meilisearch reindex jobs.</p>
        </div>
        <button
          className="button"
          onClick={() => {
            void servicesQuery.refetch();
            void auditQuery.refetch();
          }}
        >
          Refresh
        </button>
      </div>

      <div className="grid">
        <div className="card">
          <div className="row">
            <div>
              <h3>Index Runtime Config</h3>
              <p>Consumer/reindex/expiration settings used by cn-index.</p>
            </div>
            {indexService && <StatusBadge status={indexService.health?.status ?? 'unknown'} />}
          </div>
          {!indexService && <div className="notice">Index service config is unavailable.</div>}
          {indexService && (
            <>
              <div className="muted">
                Version {indexService.version} | Updated {formatTimestamp(indexService.updated_at)} by{' '}
                {indexService.updated_by}
              </div>
              <div className="field">
                <label>Config JSON</label>
                <textarea
                  rows={16}
                  value={indexDraft}
                  onChange={(event) => setIndexDraft(event.target.value)}
                />
              </div>
              {indexMessage && <div className="notice">{indexMessage}</div>}
              <button
                className="button"
                onClick={saveIndexConfig}
                disabled={saveMutation.isPending}
              >
                {saveMutation.isPending ? 'Saving...' : 'Save index config'}
              </button>
            </>
          )}
        </div>

        <div className="card">
          <h3>Reindex Job</h3>
          <p>Queue a reindex for all topics or a specific topic.</p>
          <div className="field">
            <label htmlFor="reindex-topic-id">Topic ID (optional)</label>
            <input
              id="reindex-topic-id"
              value={topicId}
              onChange={(event) => setTopicId(event.target.value)}
              placeholder="kukuri:topic:example"
            />
          </div>
          {reindexMessage && <div className="notice">{reindexMessage}</div>}
          {reindexResult && (
            <div className="card sub-card">
              <div className="row">
                <div>
                  <strong>Job {reindexResult.job_id}</strong>
                  <div className="muted">Topic {topicId.trim() === '' ? 'all topics' : topicId.trim()}</div>
                </div>
                <StatusBadge status={reindexResult.status} />
              </div>
            </div>
          )}
          <button className="button" onClick={submitReindex} disabled={reindexMutation.isPending}>
            {reindexMutation.isPending ? 'Requesting...' : 'Start reindex'}
          </button>
        </div>
      </div>

      <div className="card">
        <h3>Reindex Audit Logs</h3>
        {auditQuery.isLoading && <div className="notice">Loading audit logs...</div>}
        {auditQuery.error && <div className="notice">{errorToMessage(auditQuery.error)}</div>}
        <table className="table">
          <thead>
            <tr>
              <th>Time</th>
              <th>Actor</th>
              <th>Target</th>
              <th>Diff</th>
            </tr>
          </thead>
          <tbody>
            {(auditQuery.data ?? []).map((log) => (
              <tr key={log.audit_id}>
                <td>{formatTimestamp(log.created_at)}</td>
                <td>{log.actor_admin_user_id}</td>
                <td>{log.target}</td>
                <td>{log.diff_json ? <pre className="code-block">{formatJson(log.diff_json)}</pre> : '—'}</td>
              </tr>
            ))}
            {(auditQuery.data ?? []).length === 0 && (
              <tr>
                <td colSpan={4}>No reindex audit logs found.</td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </>
  );
};
