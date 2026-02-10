import { useMemo } from 'react';
import { useQuery } from '@tanstack/react-query';

import { Button, Card, CardContent, CardHeader, CardTitle, Notice } from '../components/ui';
import { StatusBadge } from '../components/StatusBadge';
import { api } from '../lib/api';
import { errorToMessage } from '../lib/errorHandler';
import { formatTimestamp, formatJson } from '../lib/format';
import type { DashboardSnapshot, ServiceInfo } from '../lib/types';

const numberFormatter = new Intl.NumberFormat('en-US');

const formatCount = (value?: number | null) => {
  if (typeof value !== 'number') {
    return '—';
  }
  return numberFormatter.format(value);
};

const formatSignedCount = (value?: number | null) => {
  if (typeof value !== 'number') {
    return '—';
  }
  if (value > 0) {
    return `+${numberFormatter.format(value)}`;
  }
  return numberFormatter.format(value);
};

const formatRatePerMinute = (value?: number | null) => {
  if (typeof value !== 'number') {
    return '—';
  }
  return `${value.toFixed(1)}/min`;
};

const formatPercent = (value?: number | null) => {
  if (typeof value !== 'number') {
    return '—';
  }
  return `${(value * 100).toFixed(1)}%`;
};

const formatBytes = (value?: number | null) => {
  if (typeof value !== 'number' || value < 0) {
    return '—';
  }
  const gib = value / (1024 * 1024 * 1024);
  if (gib >= 1) {
    return `${gib.toFixed(2)} GiB`;
  }
  const mib = value / (1024 * 1024);
  return `${mib.toFixed(1)} MiB`;
};

const buildRunbookAlerts = (snapshot?: DashboardSnapshot): string[] => {
  if (!snapshot) {
    return [];
  }

  const alerts: string[] = [];
  const outbox = snapshot.outbox_backlog;
  if (outbox.alert) {
    alerts.push(
      `outbox backlog=${formatCount(outbox.max_backlog)} (threshold=${formatCount(outbox.threshold)})`
    );
  }

  const reject = snapshot.reject_surge;
  if (reject.alert) {
    alerts.push(
      `reject surge=${formatRatePerMinute(reject.per_minute)} (threshold=${formatRatePerMinute(
        reject.threshold_per_minute
      )})`
    );
  }

  const db = snapshot.db_pressure;
  if (db.alert) {
    const reasons = db.alerts.length > 0 ? db.alerts.join(', ') : 'unknown';
    alerts.push(`db pressure=${reasons}`);
  }

  return alerts;
};

export const DashboardPage = () => {
  const servicesQuery = useQuery<ServiceInfo[]>({
    queryKey: ['services'],
    queryFn: api.services
  });
  const dashboardQuery = useQuery<DashboardSnapshot>({
    queryKey: ['dashboard'],
    queryFn: api.dashboard
  });

  const services = servicesQuery.data ?? [];
  const dashboard = dashboardQuery.data;

  const summary = useMemo(() => {
    const counts = { healthy: 0, degraded: 0, unreachable: 0, unknown: 0 };
    for (const service of services) {
      const status = service.health?.status ?? 'unknown';
      if (status in counts) {
        counts[status as keyof typeof counts] += 1;
      } else {
        counts.unknown += 1;
      }
    }
    return counts;
  }, [services]);

  const runbookAlerts = useMemo(() => buildRunbookAlerts(dashboard), [dashboard]);

  const outbox = dashboard?.outbox_backlog;
  const reject = dashboard?.reject_surge;
  const db = dashboard?.db_pressure;
  const topConsumer = outbox?.consumers[0];

  const outboxStatus = outbox ? (outbox.alert ? 'degraded' : 'healthy') : 'unknown';
  const rejectStatus = reject ? (reject.alert ? 'error' : reject.source_status) : 'unknown';
  const dbStatus = db ? (db.alert ? 'degraded' : 'healthy') : 'unknown';

  const isLoading = servicesQuery.isLoading || dashboardQuery.isLoading;

  const refreshAll = async () => {
    await Promise.all([servicesQuery.refetch(), dashboardQuery.refetch()]);
  };

  return (
    <>
      <div className="hero">
        <div>
          <h1>Dashboard</h1>
          <p>Health overview for community node services.</p>
        </div>
        <Button onClick={refreshAll}>
          Refresh
        </Button>
      </div>
      <div className="grid">
        <Card>
          <CardHeader>
            <CardTitle>Healthy</CardTitle>
          </CardHeader>
          <CardContent>
            <p>{summary.healthy}</p>
          </CardContent>
        </Card>
        <Card>
          <CardHeader>
            <CardTitle>Degraded</CardTitle>
          </CardHeader>
          <CardContent>
            <p>{summary.degraded}</p>
          </CardContent>
        </Card>
        <Card>
          <CardHeader>
            <CardTitle>Unreachable</CardTitle>
          </CardHeader>
          <CardContent>
            <p>{summary.unreachable}</p>
          </CardContent>
        </Card>
        <Card>
          <CardHeader>
            <CardTitle>Unknown</CardTitle>
          </CardHeader>
          <CardContent>
            <p>{summary.unknown}</p>
          </CardContent>
        </Card>
      </div>
      <div className="grid">
        <Card>
          <CardContent className="stack">
            <div className="row">
              <div>
                <h3>Outbox backlog</h3>
                <p>Max and total backlog by consumer</p>
              </div>
              <StatusBadge
                status={outboxStatus}
                label={outbox ? (outbox.alert ? 'Alert' : 'Stable') : 'No data'}
              />
            </div>
            <div className="muted">Max backlog {formatCount(outbox?.max_backlog)}</div>
            <div className="muted">Total backlog {formatCount(outbox?.total_backlog)}</div>
            <div className="muted">Threshold {formatCount(outbox?.threshold)}</div>
            <div className="muted">
              Top consumer {topConsumer ? `${topConsumer.consumer} (${formatCount(topConsumer.backlog)})` : '—'}
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="stack">
            <div className="row">
              <div>
                <h3>Reject surge</h3>
                <p>Relay reject growth from metrics delta</p>
              </div>
              <StatusBadge
                status={rejectStatus}
                label={reject ? (reject.alert ? 'Alert' : reject.source_status) : 'No data'}
              />
            </div>
            <div className="muted">Current total {formatCount(reject?.current_total)}</div>
            <div className="muted">Delta {formatSignedCount(reject?.delta)}</div>
            <div className="muted">Per minute {formatRatePerMinute(reject?.per_minute)}</div>
            <div className="muted">
              Threshold {formatRatePerMinute(reject?.threshold_per_minute)}
            </div>
            <div className="muted">
              Source {reject?.source_status ?? 'unknown'}
              {reject?.source_error ? ` (${reject.source_error})` : ''}
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="stack">
            <div className="row">
              <div>
                <h3>DB pressure</h3>
                <p>Disk, connections and lock wait pressure</p>
              </div>
              <StatusBadge
                status={dbStatus}
                label={db ? (db.alert ? 'Alert' : 'Stable') : 'No data'}
              />
            </div>
            <div className="muted">
              Size {formatBytes(db?.db_size_bytes)} / limit {formatBytes(db?.disk_soft_limit_bytes)}
            </div>
            <div className="muted">Disk utilization {formatPercent(db?.disk_utilization)}</div>
            <div className="muted">
              Connections {formatCount(db?.active_connections)} / {formatCount(db?.max_connections)} (
              {formatPercent(db?.connection_utilization)})
            </div>
            <div className="muted">
              Lock waiters {formatCount(db?.lock_waiters)} (threshold {formatCount(db?.lock_waiter_threshold)})
            </div>
          </CardContent>
        </Card>
      </div>
      {runbookAlerts.map((alert) => (
        <Notice key={alert} tone="error">
          Runbook alert: {alert}
        </Notice>
      ))}
      {isLoading && <Notice>Loading services...</Notice>}
      {servicesQuery.error && <Notice tone="error">{errorToMessage(servicesQuery.error)}</Notice>}
      {dashboardQuery.error && <Notice tone="error">{errorToMessage(dashboardQuery.error)}</Notice>}
      <div className="grid">
        {services.map((service) => (
          <Card key={service.service}>
            <CardContent className="stack">
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
              {service.health?.details != null && (
                <pre className="code-block">{formatJson(service.health.details)}</pre>
              )}
            </CardContent>
          </Card>
        ))}
      </div>
    </>
  );
};
