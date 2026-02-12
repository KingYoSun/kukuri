import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { api } from '../lib/api';
import { useAuthStore } from '../store/authStore';
import { renderWithQueryClient } from '../test/renderWithQueryClient';
import { LoginPage } from './LoginPage';

vi.mock('../lib/api', () => ({
  api: {
    me: vi.fn(),
    login: vi.fn(),
    logout: vi.fn()
  }
}));

const adminUser = {
  admin_user_id: 'admin-1',
  username: 'admin'
};

describe('LoginPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useAuthStore.setState({ user: null, status: 'unauthenticated', error: undefined });
  });

  it('ログイン成功時に認証 API を呼び出し、パスワード入力をクリアする', async () => {
    vi.mocked(api.login).mockResolvedValue(adminUser);

    renderWithQueryClient(<LoginPage />);

    const user = userEvent.setup();
    await user.type(screen.getByLabelText('Username'), 'admin');
    await user.type(screen.getByLabelText('Password'), 'password123');
    await user.click(screen.getByRole('button', { name: 'Sign in' }));

    await waitFor(() => {
      expect(api.login).toHaveBeenCalledWith('admin', 'password123');
      expect(useAuthStore.getState().status).toBe('authenticated');
    });
    expect((screen.getByLabelText('Password') as HTMLInputElement).value).toBe('');
  });

  it('ログイン失敗時にエラーメッセージを表示する', async () => {
    vi.mocked(api.login).mockRejectedValue(new Error('Invalid credentials'));

    renderWithQueryClient(<LoginPage />);

    const user = userEvent.setup();
    await user.type(screen.getByLabelText('Username'), 'admin');
    await user.type(screen.getByLabelText('Password'), 'wrong-password');
    await user.click(screen.getByRole('button', { name: 'Sign in' }));

    await waitFor(() => {
      expect(api.login).toHaveBeenCalledWith('admin', 'wrong-password');
      expect(useAuthStore.getState().status).toBe('unauthenticated');
    });
    expect(await screen.findByText('Invalid credentials')).toBeInTheDocument();
  });
});
