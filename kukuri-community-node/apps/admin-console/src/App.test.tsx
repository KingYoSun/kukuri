import type { ReactNode } from 'react';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import App from './App';
import { api } from './lib/api';
import { useAuthStore } from './store/authStore';
import { renderWithQueryClient } from './test/renderWithQueryClient';

vi.mock('./lib/api', () => ({
  api: {
    me: vi.fn(),
    login: vi.fn(),
    logout: vi.fn()
  }
}));

vi.mock('@tanstack/react-router', () => ({
  Link: ({ children }: { children: ReactNode }) => <span>{children}</span>,
  Outlet: () => <div data-testid="app-outlet" />
}));

const adminUser = {
  admin_user_id: 'admin-1',
  username: 'admin'
};

describe('App auth flow', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useAuthStore.setState({ user: null, status: 'unknown', error: undefined });
    vi.mocked(api.logout).mockResolvedValue({ status: 'ok' });
  });

  it('セッションブートストラップで未認証時はログイン画面を表示する', async () => {
    vi.mocked(api.me).mockRejectedValue(Object.assign(new Error('Unauthorized'), { status: 401 }));

    renderWithQueryClient(<App />);

    expect(screen.getByText('Checking session...')).toBeInTheDocument();
    expect(await screen.findByRole('heading', { name: 'Admin Login' })).toBeInTheDocument();
    expect(api.me).toHaveBeenCalledTimes(1);
  });

  it('セッションが有効な場合はサインアウト後にログイン画面へ遷移する', async () => {
    vi.mocked(api.me).mockResolvedValue(adminUser);

    renderWithQueryClient(<App />);

    expect(await screen.findByText('admin')).toBeInTheDocument();

    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: 'Sign out' }));

    await waitFor(() => {
      expect(api.logout).toHaveBeenCalledTimes(1);
    });
    expect(await screen.findByRole('heading', { name: 'Admin Login' })).toBeInTheDocument();
  });
});
