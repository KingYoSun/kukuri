import { fireEvent, screen, waitFor } from '@testing-library/react';
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
          retention: { events_days: 30 }
        },
        updated_at: 1738809600,
        updated_by: 'admin',
        health: { status: 'healthy', checked_at: 1738809600, details: null }
      }
    ]);
    vi.mocked(api.updateServiceConfig).mockResolvedValue({
      service: 'relay',
      version: 4,
      config_json: {
        retention: { events_days: 14 }
      },
      updated_at: 1738809601,
      updated_by: 'admin'
    });
  });

  it('表示崩れを防ぐ主要要素を描画し、設定保存を実行できる', async () => {
    renderWithQueryClient(<ServicesPage />);

    expect(await screen.findByRole('heading', { name: 'Services' })).toBeInTheDocument();
    expect(
      screen.getByText('Update runtime configuration and monitor status.')
    ).toBeInTheDocument();
    expect(await screen.findByRole('heading', { name: 'relay' })).toBeInTheDocument();
    expect(screen.getByText('Version 3')).toBeInTheDocument();
    expect(screen.getByText('healthy')).toBeInTheDocument();

    const configEditor = screen.getByLabelText('Config JSON');
    const user = userEvent.setup();
    await user.clear(configEditor);
    fireEvent.change(configEditor, { target: { value: '{invalid' } });
    await user.click(screen.getByRole('button', { name: 'Save config' }));

    expect(await screen.findByText('Config must be valid JSON.')).toBeInTheDocument();
    expect(api.updateServiceConfig).not.toHaveBeenCalled();

    fireEvent.change(configEditor, {
      target: { value: '{"retention":{"events_days":14}}' }
    });
    await user.click(screen.getByRole('button', { name: 'Save config' }));

    await waitFor(() => {
      expect(api.updateServiceConfig).toHaveBeenCalledWith(
        'relay',
        { retention: { events_days: 14 } },
        3
      );
    });
  });
});
