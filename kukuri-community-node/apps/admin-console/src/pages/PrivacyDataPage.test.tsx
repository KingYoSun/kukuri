import { fireEvent, screen, waitFor } from '@testing-library/react';
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
    updateServiceConfig: vi.fn()
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
  });
});
