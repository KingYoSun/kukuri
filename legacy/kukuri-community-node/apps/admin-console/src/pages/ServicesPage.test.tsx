import { fireEvent, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { api } from '../lib/api';
import { ServicesPage } from './ServicesPage';
import { renderWithQueryClient } from '../test/renderWithQueryClient';

vi.mock('../lib/api', () => ({
  api: {
    services: vi.fn(),
    updateServiceConfig: vi.fn()
  }
}));

describe('ServicesPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(api.services).mockResolvedValue([
      {
        service: 'relay',
        version: 3,
        config_json: {
          auth: {
            mode: 'off',
            enforce_at: null,
            grace_seconds: 900,
            ws_auth_timeout_seconds: 10
          },
          retention: { events_days: 30 }
        },
        updated_at: 1738809600,
        updated_by: 'admin',
        health: {
          status: 'healthy',
          checked_at: 1738809600,
          details: {
            status: 200,
            auth_transition: {
              metrics_status: 200,
              ws_connections: 9,
              ws_unauthenticated_connections: 3,
              ingest_rejected_auth_total: 7,
              ws_auth_disconnect_timeout_total: 2,
              ws_auth_disconnect_deadline_total: 1
            }
          }
        }
      },
      {
        service: 'bootstrap',
        version: 2,
        config_json: {
          auth: {
            mode: 'required',
            enforce_at: null,
            grace_seconds: 600,
            ws_auth_timeout_seconds: 8
          }
        },
        updated_at: 1738809601,
        updated_by: 'admin',
        health: { status: 'degraded', checked_at: 1738809601, details: { status: 503 } }
      },
      {
        service: 'index',
        version: 1,
        config_json: {
          indexing: { enabled: true }
        },
        updated_at: 1738809602,
        updated_by: 'admin',
        health: { status: 'healthy', checked_at: 1738809602, details: null }
      }
    ]);
    vi.mocked(api.updateServiceConfig).mockResolvedValue({
      service: 'relay',
      version: 4,
      config_json: {},
      updated_at: 1738809700,
      updated_by: 'admin'
    });
  });

  it('relay auth 遷移フォームを保存し、施行状態の主要メトリクスを表示する', async () => {
    renderWithQueryClient(<ServicesPage />);

    expect(await screen.findByRole('heading', { name: 'Services' })).toBeInTheDocument();
    const relayHeading = await screen.findByRole('heading', { name: 'relay Auth Transition' });
    const relayCard = relayHeading.closest('.card');
    expect(relayCard).not.toBeNull();
    const relayScope = within(relayCard as HTMLElement);

    expect(relayScope.getByText('Unauthenticated connections remaining')).toBeInTheDocument();
    expect(relayScope.getByText('Auth-required rejects total')).toBeInTheDocument();
    const unauthRow = relayScope
      .getByText('Unauthenticated connections remaining')
      .closest('tr');
    expect(unauthRow).not.toBeNull();
    expect(within(unauthRow as HTMLElement).getByText('3')).toBeInTheDocument();
    const rejectRow = relayScope.getByText('Auth-required rejects total').closest('tr');
    expect(rejectRow).not.toBeNull();
    expect(within(rejectRow as HTMLElement).getByText('7')).toBeInTheDocument();

    const user = userEvent.setup();
    await user.selectOptions(relayScope.getByLabelText('Auth mode'), 'required');
    await user.selectOptions(relayScope.getByLabelText('Enforce timing'), 'scheduled');
    fireEvent.change(relayScope.getByLabelText('Enforce at'), {
      target: { value: '2026-02-13T12:30' }
    });
    fireEvent.change(relayScope.getByLabelText('Grace seconds'), { target: { value: '1200' } });
    fireEvent.change(relayScope.getByLabelText('WS auth timeout seconds'), {
      target: { value: '12' }
    });
    await user.click(relayScope.getByRole('button', { name: 'Save auth transition' }));

    const expectedEnforceAt = Math.floor(new Date('2026-02-13T12:30').getTime() / 1000);
    await waitFor(() => {
      expect(api.updateServiceConfig).toHaveBeenCalledWith(
        'relay',
        expect.objectContaining({
          retention: { events_days: 30 },
          auth: expect.objectContaining({
            mode: 'required',
            enforce_at: expectedEnforceAt,
            grace_seconds: 1200,
            ws_auth_timeout_seconds: 12
          })
        }),
        3
      );
    });
  });

  it('relay auth 遷移フォームは不正入力を保存しない', async () => {
    renderWithQueryClient(<ServicesPage />);

    const relayHeading = await screen.findByRole('heading', { name: 'relay Auth Transition' });
    const relayCard = relayHeading.closest('.card');
    expect(relayCard).not.toBeNull();
    const relayScope = within(relayCard as HTMLElement);

    const user = userEvent.setup();
    await user.selectOptions(relayScope.getByLabelText('Auth mode'), 'required');
    await user.selectOptions(relayScope.getByLabelText('Enforce timing'), 'scheduled');
    fireEvent.change(relayScope.getByLabelText('Enforce at'), { target: { value: '' } });
    await user.click(relayScope.getByRole('button', { name: 'Save auth transition' }));

    expect(await relayScope.findByText('Enforce at must be a valid datetime.')).toBeInTheDocument();
    expect(api.updateServiceConfig).not.toHaveBeenCalled();
  });

  it('bootstrap auth 遷移フォームの初期表示と保存 payload 契約を維持する', async () => {
    renderWithQueryClient(<ServicesPage />);

    const bootstrapHeading = await screen.findByRole('heading', { name: 'bootstrap Auth Transition' });
    const bootstrapCard = bootstrapHeading.closest('.card');
    expect(bootstrapCard).not.toBeNull();
    const bootstrapScope = within(bootstrapCard as HTMLElement);

    expect(bootstrapScope.getByLabelText('Auth mode')).toHaveValue('required');
    expect(bootstrapScope.getByLabelText('Enforce timing')).toHaveValue('immediate');
    expect(bootstrapScope.getByLabelText('Grace seconds')).toHaveValue(600);
    expect(bootstrapScope.getByLabelText('WS auth timeout seconds')).toHaveValue(8);
    expect(bootstrapScope.queryByText('Relay Runtime Signals')).not.toBeInTheDocument();

    const user = userEvent.setup();
    await user.selectOptions(bootstrapScope.getByLabelText('Enforce timing'), 'scheduled');
    fireEvent.change(bootstrapScope.getByLabelText('Enforce at'), {
      target: { value: '2026-02-14T09:45' }
    });
    fireEvent.change(bootstrapScope.getByLabelText('Grace seconds'), { target: { value: '1800' } });
    fireEvent.change(bootstrapScope.getByLabelText('WS auth timeout seconds'), {
      target: { value: '15' }
    });
    await user.click(bootstrapScope.getByRole('button', { name: 'Save auth transition' }));

    const expectedEnforceAt = Math.floor(new Date('2026-02-14T09:45').getTime() / 1000);
    await waitFor(() => {
      expect(api.updateServiceConfig).toHaveBeenCalledWith(
        'bootstrap',
        expect.objectContaining({
          auth: expect.objectContaining({
            mode: 'required',
            enforce_at: expectedEnforceAt,
            grace_seconds: 1800,
            ws_auth_timeout_seconds: 15
          })
        }),
        2
      );
    });
  });

  it('bootstrap auth 遷移フォームは不正入力を保存しない', async () => {
    renderWithQueryClient(<ServicesPage />);

    const bootstrapHeading = await screen.findByRole('heading', { name: 'bootstrap Auth Transition' });
    const bootstrapCard = bootstrapHeading.closest('.card');
    expect(bootstrapCard).not.toBeNull();
    const bootstrapScope = within(bootstrapCard as HTMLElement);

    const user = userEvent.setup();
    await user.selectOptions(bootstrapScope.getByLabelText('Enforce timing'), 'scheduled');
    fireEvent.change(bootstrapScope.getByLabelText('Enforce at'), { target: { value: '' } });
    await user.click(bootstrapScope.getByRole('button', { name: 'Save auth transition' }));

    expect(await bootstrapScope.findByText('Enforce at must be a valid datetime.')).toBeInTheDocument();
    expect(api.updateServiceConfig).not.toHaveBeenCalled();
  });

  it('relay/bootstrap 以外は従来の JSON 編集と秘匿キー検証を維持する', async () => {
    renderWithQueryClient(<ServicesPage />);

    const indexHeading = await screen.findByRole('heading', { name: 'index' });
    const indexCard = indexHeading.closest('.card');
    expect(indexCard).not.toBeNull();
    const indexScope = within(indexCard as HTMLElement);

    const configEditor = indexScope.getByLabelText('Config JSON');
    fireEvent.change(configEditor, {
      target: {
        value: '{"llm":{"provider":"openai","OPENAI_API_KEY":"sk-test"}}'
      }
    });

    const user = userEvent.setup();
    await user.click(indexScope.getByRole('button', { name: 'Save config' }));

    expect(
      await indexScope.findByText(/Secret keys are not allowed in service config:/)
    ).toBeInTheDocument();
    expect(api.updateServiceConfig).not.toHaveBeenCalled();
  });
});
