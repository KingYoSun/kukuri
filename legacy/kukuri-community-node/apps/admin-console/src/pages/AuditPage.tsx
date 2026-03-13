import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';

import { api } from '../lib/api';
import { errorToMessage } from '../lib/errorHandler';
import { formatJson, formatTimestamp } from '../lib/format';
import type { AuditLog, ServiceInfo } from '../lib/types';
import { StatusBadge } from '../components/StatusBadge';

export const AuditPage = () => {
  const [filters, setFilters] = useState({ action: '', target: '', since: '' });

  const servicesQuery = useQuery<ServiceInfo[]>({
    queryKey: ['services'],
    queryFn: api.services
  });

  const auditQuery = useQuery<AuditLog[]>({
    queryKey: ['auditLogs', filters.action, filters.target, filters.since],
    queryFn: () =>
      api.auditLogs({
        action: filters.action || undefined,
        target: filters.target || undefined,
        since: filters.since ? Number(filters.since) : undefined,
        limit: 200
      })
  });

  return (
    <>
      <div className="hero">
        <div>
          <h1>Audit & Health</h1>
          <p>Service health snapshots and admin audit logs.</p>
        </div>
      </div>

      <div className="card">
        <h3>Service Health</h3>
        {servicesQuery.isLoading && <div className="notice">Loading health...</div>}
        {servicesQuery.error && (
          <div className="notice">{errorToMessage(servicesQuery.error)}</div>
        )}
        <div className="grid">
          {(servicesQuery.data ?? []).map((service) => (
            <div key={service.service} className="card sub-card">
              <div className="row">
                <strong>{service.service}</strong>
                <StatusBadge status={service.health?.status ?? 'unknown'} />
              </div>
              <div className="muted">
                Checked {formatTimestamp(service.health?.checked_at ?? null)}
              </div>
              {service.health?.details != null && (
                <pre className="code-block">{formatJson(service.health.details)}</pre>
              )}
            </div>
          ))}
          {(servicesQuery.data ?? []).length === 0 && (
            <div className="notice">No health data available.</div>
          )}
        </div>
      </div>

      <div className="card">
        <div className="row">
          <div>
            <h3>Audit Logs</h3>
            <p>Last 200 entries with optional filters.</p>
          </div>
        </div>
        <div className="grid">
          <div className="field">
            <label>Action</label>
            <input
              value={filters.action}
              onChange={(event) =>
                setFilters((prev) => ({ ...prev, action: event.target.value }))
              }
              placeholder="policy.publish"
            />
          </div>
          <div className="field">
            <label>Target</label>
            <input
              value={filters.target}
              onChange={(event) =>
                setFilters((prev) => ({ ...prev, target: event.target.value }))
              }
              placeholder="service:relay"
            />
          </div>
          <div className="field">
            <label>Since (epoch seconds)</label>
            <input
              value={filters.since}
              onChange={(event) =>
                setFilters((prev) => ({ ...prev, since: event.target.value }))
              }
              placeholder="1730000000"
            />
          </div>
        </div>
        {auditQuery.isLoading && <div className="notice">Loading audit logs...</div>}
        {auditQuery.error && <div className="notice">{errorToMessage(auditQuery.error)}</div>}
        <table className="table">
          <thead>
            <tr>
              <th>Time</th>
              <th>Actor</th>
              <th>Action</th>
              <th>Target</th>
              <th>Diff</th>
            </tr>
          </thead>
          <tbody>
            {(auditQuery.data ?? []).map((log) => (
              <tr key={log.audit_id}>
                <td>{formatTimestamp(log.created_at)}</td>
                <td>{log.actor_admin_user_id}</td>
                <td>{log.action}</td>
                <td>{log.target}</td>
                <td>
                  {log.diff_json ? (
                    <pre className="code-block">{formatJson(log.diff_json)}</pre>
                  ) : (
                    '—'
                  )}
                </td>
              </tr>
            ))}
            {(auditQuery.data ?? []).length === 0 && (
              <tr>
                <td colSpan={5}>No audit logs found.</td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </>
  );
};
