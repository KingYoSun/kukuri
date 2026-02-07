import { fireEvent, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { api } from '../lib/api';
import { AuditPage } from './AuditPage';
import { renderWithQueryClient } from '../test/renderWithQueryClient';

vi.mock('../lib/api', () => ({
  api: {
    services: vi.fn(),
    auditLogs: vi.fn()
  }
}));

describe('AuditPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(api.services).mockResolvedValue([
      {
        service: 'relay',
        version: 2,
        config_json: {},
        updated_at: 1738809600,
        updated_by: 'admin',
        health: { status: 'healthy', checked_at: 1738809600, details: { uptime: 99.7 } }
      }
    ]);
    vi.mocked(api.auditLogs).mockResolvedValue([
      {
        audit_id: 'audit-1',
        actor_admin_user_id: 'admin-1',
        action: 'policy.publish',
        target: 'policy:terms:2026-01',
        diff_json: { previous: 'draft', next: 'published' },
        created_at: 1738809600
      }
    ]);
  });

  it('ヘルス表示と監査ログフィルタを検証できる', async () => {
    renderWithQueryClient(<AuditPage />);

    expect(await screen.findByRole('heading', { name: 'Audit & Health' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Service Health' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Audit Logs' })).toBeInTheDocument();
    expect(screen.getByRole('columnheader', { name: 'Action' })).toBeInTheDocument();
    expect(screen.getByRole('columnheader', { name: 'Diff' })).toBeInTheDocument();
    expect(await screen.findByText('relay')).toBeInTheDocument();
    expect(await screen.findByText('policy.publish')).toBeInTheDocument();
    expect(screen.getByText(/"next": "published"/)).toBeInTheDocument();

    fireEvent.change(screen.getByPlaceholderText('policy.publish'), {
      target: { value: 'service.update' }
    });
    fireEvent.change(screen.getByPlaceholderText('service:relay'), {
      target: { value: 'service:relay' }
    });
    fireEvent.change(screen.getByPlaceholderText('1730000000'), {
      target: { value: '1730000000' }
    });

    await waitFor(() => {
      expect(api.auditLogs).toHaveBeenLastCalledWith({
        action: 'service.update',
        target: 'service:relay',
        since: 1730000000,
        limit: 200
      });
    });
  });
});
