import { screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { api } from '../lib/api';
import { DashboardPage } from './DashboardPage';
import { renderWithQueryClient } from '../test/renderWithQueryClient';

vi.mock('../lib/api', () => ({
  api: {
    services: vi.fn(),
    dashboard: vi.fn()
  }
}));

describe('DashboardPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(api.services).mockResolvedValue([
      {
        service: 'relay',
        version: 1,
        config_json: {},
        updated_at: 1738809600,
        updated_by: 'admin',
        health: { status: 'healthy', checked_at: 1738809600, details: { uptime: 99.9 } }
      },
      {
        service: 'index',
        version: 2,
        config_json: {},
        updated_at: 1738809601,
        updated_by: 'admin',
        health: { status: 'degraded', checked_at: 1738809601, details: null }
      },
      {
        service: 'bootstrap',
        version: 3,
        config_json: {},
        updated_at: 1738809602,
        updated_by: 'admin',
        health: { status: 'unreachable', checked_at: 1738809602, details: null }
      },
      {
        service: 'user-api',
        version: 4,
        config_json: {},
        updated_at: 1738809603,
        updated_by: 'admin',
        health: { status: 'unexpected', checked_at: 1738809603, details: null }
      }
    ]);
    vi.mocked(api.dashboard).mockResolvedValue({
      collected_at: 1738809604,
      outbox_backlog: {
        max_seq: 2500,
        total_backlog: 1400,
        max_backlog: 1200,
        threshold: 1000,
        alert: true,
        consumers: [
          { consumer: 'cn-index', last_seq: 1300, backlog: 1200 },
          { consumer: 'cn-trust', last_seq: 2300, backlog: 200 }
        ]
      },
      reject_surge: {
        source_status: 'ok',
        source_error: null,
        current_total: 420,
        previous_total: 360,
        delta: 60,
        per_minute: 45,
        threshold_per_minute: 30,
        alert: true
      },
      db_pressure: {
        db_size_bytes: 11811160064,
        disk_soft_limit_bytes: 10737418240,
        disk_utilization: 1.1,
        active_connections: 92,
        max_connections: 100,
        connection_utilization: 0.92,
        lock_waiters: 4,
        connection_threshold: 0.85,
        lock_waiter_threshold: 3,
        alert: true,
        alerts: ['disk_soft_limit_exceeded', 'connections_near_capacity']
      }
    });
  });

  it('Runbook 指標とサービス一覧を表示し、Refresh で再取得できる', async () => {
    renderWithQueryClient(<DashboardPage />);

    expect(await screen.findByRole('heading', { name: 'Dashboard' })).toBeInTheDocument();
    expect(screen.getByText('Health overview for community node services.')).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Outbox backlog' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Reject surge' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'DB pressure' })).toBeInTheDocument();
    expect(
      await screen.findByText('Runbook alert: outbox backlog=1,200 (threshold=1,000)')
    ).toBeInTheDocument();
    expect(
      screen.getByText('Runbook alert: reject surge=45.0/min (threshold=30.0/min)')
    ).toBeInTheDocument();
    expect(
      screen.getByText('Runbook alert: db pressure=disk_soft_limit_exceeded, connections_near_capacity')
    ).toBeInTheDocument();
    expect(screen.getByText('Top consumer cn-index (1,200)')).toBeInTheDocument();
    expect(screen.getByText('Current total 420')).toBeInTheDocument();
    expect(screen.getByText('Delta +60')).toBeInTheDocument();
    expect(screen.getByText('Per minute 45.0/min')).toBeInTheDocument();

    const healthyCard = screen.getByRole('heading', { name: 'Healthy' }).closest('.card');
    const degradedCard = screen.getByRole('heading', { name: 'Degraded' }).closest('.card');
    const unreachableCard = screen.getByRole('heading', { name: 'Unreachable' }).closest('.card');
    const unknownCard = screen.getByRole('heading', { name: 'Unknown' }).closest('.card');

    expect(healthyCard).not.toBeNull();
    expect(degradedCard).not.toBeNull();
    expect(unreachableCard).not.toBeNull();
    expect(unknownCard).not.toBeNull();
    expect(await screen.findByRole('heading', { name: 'relay' })).toBeInTheDocument();
    expect(within(healthyCard as HTMLElement).getByText('1')).toBeInTheDocument();
    expect(within(degradedCard as HTMLElement).getByText('1')).toBeInTheDocument();
    expect(within(unreachableCard as HTMLElement).getByText('1')).toBeInTheDocument();
    expect(within(unknownCard as HTMLElement).getByText('1')).toBeInTheDocument();

    expect(screen.getByRole('heading', { name: 'index' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'bootstrap' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'user-api' })).toBeInTheDocument();
    expect(screen.getByText(/"uptime": 99.9/)).toBeInTheDocument();

    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: 'Refresh' }));

    await waitFor(() => {
      expect(api.services).toHaveBeenCalledTimes(2);
      expect(api.dashboard).toHaveBeenCalledTimes(2);
    });
  });
});
