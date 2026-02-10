import { fireEvent, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { api } from '../lib/api';
import { PrivacyDataPage } from './PrivacyDataPage';
import { renderWithQueryClient } from '../test/renderWithQueryClient';

vi.mock('../lib/api', () => ({
  api: {
    services: vi.fn(),
    policies: vi.fn(),
    auditLogs: vi.fn(),
    updateServiceConfig: vi.fn(),
    dsarJobs: vi.fn(),
    retryDsarJob: vi.fn(),
    cancelDsarJob: vi.fn()
  }
}));

describe('PrivacyDataPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(api.services).mockResolvedValue([
      {
        service: 'relay',
        version: 3,
        config_json: {
          retention: {
            events_days: 30,
            outbox_days: 30,
            dedupe_days: 180,
            tombstone_days: 180,
            cleanup_interval_seconds: 3600
          }
        },
        updated_at: 1738809600,
        updated_by: 'admin',
        health: { status: 'healthy', checked_at: 1738809600, details: null }
      },
      {
        service: 'user-api',
        version: 2,
        config_json: {
          rate_limit: {
            auth_per_minute: 20,
            public_per_minute: 120,
            protected_per_minute: 120
          }
        },
        updated_at: 1738809600,
        updated_by: 'admin',
        health: { status: 'healthy', checked_at: 1738809600, details: null }
      }
    ]);
    vi.mocked(api.policies).mockResolvedValue([
      {
        policy_id: 'terms-1',
        policy_type: 'terms',
        version: '2026-01',
        locale: 'ja-JP',
        title: 'Terms',
        content_md: 'terms',
        content_hash: 'hash-terms',
        published_at: 1738809600,
        effective_at: 1738809600,
        is_current: true
      },
      {
        policy_id: 'privacy-1',
        policy_type: 'privacy',
        version: '2026-01',
        locale: 'ja-JP',
        title: 'Privacy',
        content_md: 'privacy',
        content_hash: 'hash-privacy',
        published_at: 1738809600,
        effective_at: 1738809600,
        is_current: true
      }
    ]);
    vi.mocked(api.auditLogs).mockResolvedValue([]);
    vi.mocked(api.dsarJobs).mockResolvedValue([
      {
        job_id: 'export-1',
        request_type: 'export',
        requester_pubkey: 'a'.repeat(64),
        status: 'failed',
        created_at: 1738809600,
        completed_at: 1738809700,
        error_message: 'timeout'
      },
      {
        job_id: 'deletion-1',
        request_type: 'deletion',
        requester_pubkey: 'b'.repeat(64),
        status: 'running',
        created_at: 1738809800,
        completed_at: null,
        error_message: null
      }
    ]);
    vi.mocked(api.retryDsarJob).mockResolvedValue({
      job_id: 'export-1',
      request_type: 'export',
      requester_pubkey: 'a'.repeat(64),
      status: 'queued',
      created_at: 1738809600,
      completed_at: null,
      error_message: null
    });
    vi.mocked(api.cancelDsarJob).mockResolvedValue({
      job_id: 'deletion-1',
      request_type: 'deletion',
      requester_pubkey: 'b'.repeat(64),
      status: 'failed',
      created_at: 1738809800,
      completed_at: 1738809900,
      error_message: 'canceled by admin'
    });
    vi.mocked(api.updateServiceConfig).mockResolvedValue({
      service: 'relay',
      version: 4,
      config_json: { retention: { events_days: 30 } },
      updated_at: 1738809601,
      updated_by: 'admin'
    });
  });

  it('ポリシー情報を表示し、Relay 設定保存を実行できる', async () => {
    renderWithQueryClient(<PrivacyDataPage />);

    expect(await screen.findByRole('heading', { name: 'Privacy / Data' })).toBeInTheDocument();
    await waitFor(() => {
      expect(screen.queryByText('Loading policies...')).not.toBeInTheDocument();
      expect(screen.queryByText('Loading services...')).not.toBeInTheDocument();
    });
    expect(screen.getByText('Current Policies')).toBeInTheDocument();
    expect(screen.getAllByText('2026-01 (ja-JP)')).toHaveLength(2);
    expect(screen.getByRole('heading', { name: 'DSAR Operations' })).toBeInTheDocument();
    expect(await screen.findByText('export-1')).toBeInTheDocument();

    const user = userEvent.setup();
    const configEditors = screen.getAllByLabelText('Config JSON');
    await user.clear(configEditors[0]);
    fireEvent.change(configEditors[0], {
      target: { value: '{"retention":{"events_days":14}}' }
    });
    await user.click(screen.getAllByRole('button', { name: 'Save config' })[0]);

    await waitFor(() => {
      expect(api.updateServiceConfig).toHaveBeenCalledWith(
        'relay',
        { retention: { events_days: 14 } },
        3
      );
    });

    const exportRow = screen.getByText('export-1').closest('tr');
    expect(exportRow).not.toBeNull();
    await user.click(within(exportRow as HTMLElement).getByRole('button', { name: 'Retry' }));
    await waitFor(() => {
      expect(api.retryDsarJob).toHaveBeenCalledWith('export', 'export-1');
    });

    const deletionRow = screen.getByText('deletion-1').closest('tr');
    expect(deletionRow).not.toBeNull();
    await user.click(within(deletionRow as HTMLElement).getByRole('button', { name: 'Cancel' }));
    await waitFor(() => {
      expect(api.cancelDsarJob).toHaveBeenCalledWith('deletion', 'deletion-1');
    });

    await user.click(screen.getByRole('button', { name: 'Refresh' }));
    await waitFor(() => {
      expect(vi.mocked(api.dsarJobs).mock.calls.length).toBeGreaterThan(1);
    });
  });
});
