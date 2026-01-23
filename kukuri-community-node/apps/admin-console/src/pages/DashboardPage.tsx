import { useMemo } from 'react';
import { useQuery } from '@tanstack/react-query';

import { api } from '../lib/api';
import { errorToMessage } from '../lib/errorHandler';
import { formatTimestamp, formatJson } from '../lib/format';
import type { ServiceInfo } from '../lib/types';
import { StatusBadge } from '../components/StatusBadge';

export const DashboardPage = () => {
  const { data, isLoading, error, refetch } = useQuery<ServiceInfo[]>({
    queryKey: ['services'],
    queryFn: api.services
  });

  const summary = useMemo(() => {
    const counts = { healthy: 0, degraded: 0, unreachable: 0, unknown: 0 };
    for (const service of data ?? []) {
      const status = service.health?.status ?? 'unknown';
      if (status in counts) {
        counts[status as keyof typeof counts] += 1;
      } else {
        counts.unknown += 1;
      }
    }
    return counts;
  }, [data]);

  return (
    <>
      <div className="hero">
        <div>
          <h1>Dashboard</h1>
          <p>Health overview for community node services.</p>
        </div>
        <button className="button" onClick={() => refetch()}>
          Refresh
        </button>
      </div>
      <div className="grid">
        <div className="card">
          <h3>Healthy</h3>
          <p>{summary.healthy}</p>
        </div>
        <div className="card">
          <h3>Degraded</h3>
          <p>{summary.degraded}</p>
        </div>
        <div className="card">
          <h3>Unreachable</h3>
          <p>{summary.unreachable}</p>
        </div>
        <div className="card">
          <h3>Unknown</h3>
          <p>{summary.unknown}</p>
        </div>
      </div>
      {isLoading && <div className="notice">Loading services...</div>}
      {error && <div className="notice">{errorToMessage(error)}</div>}
      <div className="grid">
        {(data ?? []).map((service) => (
          <div key={service.service} className="card">
            <div className="stack">
              <div className="row">
                <div>
                  <h3>{service.service}</h3>
                  <p>Version {service.version}</p>
                </div>
                <StatusBadge status={service.health?.status ?? 'unknown'} />
              </div>
              <div className="muted">
                Updated {formatTimestamp(service.updated_at)} by {service.updated_by}
              </div>
              <div className="muted">
                Health checked {formatTimestamp(service.health?.checked_at ?? null)}
              </div>
              {service.health?.details && (
                <pre className="code-block">{formatJson(service.health.details)}</pre>
              )}
            </div>
          </div>
        ))}
      </div>
    </>
  );
};
