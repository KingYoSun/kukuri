import { screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { api } from '../lib/api';
import { DashboardPage } from './DashboardPage';
import { renderWithQueryClient } from '../test/renderWithQueryClient';

vi.mock('../lib/api', () => ({
  api: {
    services: vi.fn()
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
  });

  it('集計カードとサービス一覧を表示し、Refresh で再取得できる', async () => {
    renderWithQueryClient(<DashboardPage />);

    expect(await screen.findByRole('heading', { name: 'Dashboard' })).toBeInTheDocument();
    expect(screen.getByText('Health overview for community node services.')).toBeInTheDocument();

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
    });
  });
});
