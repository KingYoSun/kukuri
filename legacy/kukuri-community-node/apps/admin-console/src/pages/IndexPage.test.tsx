import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { api } from '../lib/api';
import { IndexPage } from './IndexPage';
import { renderWithQueryClient } from '../test/renderWithQueryClient';

vi.mock('../lib/api', () => ({
  api: {
    services: vi.fn(),
    auditLogs: vi.fn(),
    updateServiceConfig: vi.fn(),
    reindex: vi.fn()
  }
}));

describe('IndexPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(api.services).mockResolvedValue([
      {
        service: 'index',
        version: 1,
        config_json: {
          enabled: true,
          consumer: { batch_size: 200, poll_interval_seconds: 5 }
        },
        updated_at: 1738809600,
        updated_by: 'admin',
        health: { status: 'healthy', checked_at: 1738809600, details: null }
      }
    ]);
    vi.mocked(api.auditLogs).mockResolvedValue([]);
    vi.mocked(api.updateServiceConfig).mockResolvedValue({
      service: 'index',
      version: 2,
      config_json: {
        enabled: true,
        consumer: { batch_size: 200, poll_interval_seconds: 5 }
      },
      updated_at: 1738809601,
      updated_by: 'admin'
    });
    vi.mocked(api.reindex).mockResolvedValue({
      job_id: 'job-1',
      status: 'pending'
    });
  });

  it('トピック指定の reindex リクエストを送信できる', async () => {
    renderWithQueryClient(<IndexPage />);

    expect(await screen.findByRole('heading', { name: 'Index' })).toBeInTheDocument();

    const user = userEvent.setup();
    await user.type(screen.getByLabelText('Topic ID (optional)'), 'kukuri:topic:test');
    await user.click(screen.getByRole('button', { name: 'Start reindex' }));

    await waitFor(() => {
      expect(api.reindex).toHaveBeenCalledWith('kukuri:topic:test');
    });
    expect(await screen.findByText('Reindex request accepted.')).toBeInTheDocument();
  });
});
