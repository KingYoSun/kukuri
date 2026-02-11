import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { api } from '../lib/api';
import { ModerationPage } from './ModerationPage';
import { renderWithQueryClient } from '../test/renderWithQueryClient';

vi.mock('../lib/api', () => ({
  api: {
    services: vi.fn(),
    auditLogs: vi.fn(),
    updateServiceConfig: vi.fn(),
    moderationRules: vi.fn(),
    createModerationRule: vi.fn(),
    updateModerationRule: vi.fn(),
    deleteModerationRule: vi.fn(),
    testModerationRule: vi.fn(),
    moderationReports: vi.fn(),
    moderationLabels: vi.fn(),
    createManualLabel: vi.fn()
  }
}));

describe('ModerationPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(api.services).mockResolvedValue([
      {
        service: 'moderation',
        version: 4,
        config_json: {
          enabled: true,
          llm: {
            enabled: false,
            provider: 'disabled',
            external_send_enabled: false,
            send_scope: {
              public: true,
              invite: false,
              friend: false,
              friend_plus: false
            },
            storage: {
              persist_decisions: true,
              persist_request_snapshots: false
            },
            retention: {
              decision_days: 90,
              snapshot_days: 7
            },
            truncate_chars: 2000,
            mask_pii: true,
            max_requests_per_day: 0,
            max_cost_per_day: 0,
            max_concurrency: 1
          }
        },
        updated_at: 1738809600,
        updated_by: 'admin',
        health: { status: 'healthy', checked_at: 1738809600, details: null }
      }
    ]);
    vi.mocked(api.auditLogs).mockResolvedValue([]);
    vi.mocked(api.moderationRules).mockResolvedValue([]);
    vi.mocked(api.moderationReports).mockResolvedValue([]);
    vi.mocked(api.moderationLabels).mockResolvedValue([]);
    vi.mocked(api.updateServiceConfig).mockResolvedValue({
      service: 'moderation',
      version: 5,
      config_json: { llm: { enabled: true } },
      updated_at: 1738809601,
      updated_by: 'admin'
    });
    vi.mocked(api.createModerationRule).mockResolvedValue({
      rule_id: 'rule-1',
      name: 'rule',
      description: null,
      is_enabled: true,
      priority: 0,
      conditions: {},
      action: {},
      created_at: 1738809600,
      updated_at: 1738809600,
      updated_by: 'admin'
    });
    vi.mocked(api.updateModerationRule).mockResolvedValue({
      rule_id: 'rule-1',
      name: 'rule',
      description: null,
      is_enabled: true,
      priority: 0,
      conditions: {},
      action: {},
      created_at: 1738809600,
      updated_at: 1738809600,
      updated_by: 'admin'
    });
    vi.mocked(api.deleteModerationRule).mockResolvedValue({ status: 'deleted' });
    vi.mocked(api.testModerationRule).mockResolvedValue({
      matched: true,
      reasons: ['content keyword matched'],
      preview: {
        target: 'event:event-123',
        label: 'spam',
        confidence: 0.9,
        exp: 1738813200,
        policy_url: 'https://example.com/policy',
        policy_ref: 'moderation-v1'
      }
    });
    vi.mocked(api.createManualLabel).mockResolvedValue({ label_id: 'label-1', status: 'created' });
  });

  it('LLM 設定を専用フォームから保存できる', async () => {
    renderWithQueryClient(<ModerationPage />);
    expect(await screen.findByRole('heading', { name: 'Moderation' })).toBeInTheDocument();
    await screen.findByRole('button', { name: 'Save LLM settings' });

    const user = userEvent.setup();
    await user.selectOptions(screen.getByLabelText('LLM enabled'), 'true');
    await user.selectOptions(screen.getByLabelText('Provider'), 'local');
    await user.clear(screen.getByLabelText('Max requests per day'));
    await user.type(screen.getByLabelText('Max requests per day'), '120');
    await user.clear(screen.getByLabelText('Max cost per day'));
    await user.type(screen.getByLabelText('Max cost per day'), '5.5');
    await user.clear(screen.getByLabelText('Max concurrency'));
    await user.type(screen.getByLabelText('Max concurrency'), '3');
    await user.clear(screen.getByLabelText('Decision retention days'));
    await user.type(screen.getByLabelText('Decision retention days'), '120');
    await user.clear(screen.getByLabelText('Snapshot retention days'));
    await user.type(screen.getByLabelText('Snapshot retention days'), '14');
    await user.click(screen.getByRole('button', { name: 'Save LLM settings' }));

    await waitFor(() => {
      expect(api.updateServiceConfig).toHaveBeenCalledWith(
        'moderation',
        expect.objectContaining({
          llm: expect.objectContaining({
            enabled: true,
            provider: 'local',
            external_send_enabled: false,
            max_requests_per_day: 120,
            max_cost_per_day: 5.5,
            max_concurrency: 3,
            retention: expect.objectContaining({
              decision_days: 120,
              snapshot_days: 14
            })
          })
        }),
        4
      );
    });
  });

  it('ルールテスト実行を送信できる', async () => {
    renderWithQueryClient(<ModerationPage />);
    await screen.findByRole('button', { name: 'Run rule test' });

    const user = userEvent.setup();
    await user.type(screen.getByLabelText('Sample event id (optional)'), 'event-123');
    await user.type(screen.getByLabelText('Sample pubkey'), 'a'.repeat(64));
    await user.clear(screen.getByLabelText('Sample content'));
    await user.type(screen.getByLabelText('Sample content'), 'contains spam keyword');
    await user.click(screen.getByRole('button', { name: 'Run rule test' }));

    await waitFor(() => {
      expect(api.testModerationRule).toHaveBeenCalledWith(
        expect.objectContaining({
          conditions: expect.any(Object),
          action: expect.any(Object),
          sample: expect.objectContaining({
            event_id: 'event-123',
            pubkey: 'a'.repeat(64),
            kind: 1
          })
        })
      );
    });
  });
});
