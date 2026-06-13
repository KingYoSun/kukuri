import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, expect, test, vi } from 'vitest';

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

import { App } from '@/App';
import { DESKTOP_THEME_STORAGE_KEY } from '@/lib/theme';
import { createDesktopMockApi } from '@/mocks/desktopApiMock';

beforeEach(() => {
  Object.defineProperty(window, 'innerWidth', {
    configurable: true,
    writable: true,
    value: 1024,
  });
  window.dispatchEvent(new Event('resize'));
  window.history.replaceState(null, '', '/');
  window.localStorage.clear();
  document.documentElement.removeAttribute('data-theme');
  invokeMock.mockReset();
  delete window.__KUKURI_DESKTOP__;
});

test('desktop app bootstraps the shell with the default timeline workspace', async () => {
  render(<App api={createDesktopMockApi()} />);

  await waitFor(() => {
    expect(document.documentElement).toHaveAttribute('data-theme', 'dark');
  });
  expect(screen.getByRole('tablist', { name: 'Workspaces' })).toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'kukuri:topic:demo' })).toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'kukuri:topic:demo' }).closest('li')).toHaveClass(
    'topic-item-active'
  );
  expect(screen.getByRole('button', { name: 'Publish' })).toBeInTheDocument();
  expect(window.localStorage.getItem(DESKTOP_THEME_STORAGE_KEY)).toBe('dark');
});

test('desktop app restores a persisted light theme on boot', async () => {
  window.localStorage.setItem(DESKTOP_THEME_STORAGE_KEY, 'light');

  render(<App api={createDesktopMockApi()} />);

  await waitFor(() => {
    expect(document.documentElement).toHaveAttribute('data-theme', 'light');
  });
});

test('preview release banner opens release settings', async () => {
  const user = userEvent.setup();
  render(<App api={createDesktopMockApi()} />);

  await user.click(await screen.findByRole('button', { name: 'Updates' }));

  expect(screen.getByRole('dialog', { name: 'Settings' })).toBeInTheDocument();
  expect(screen.getByRole('heading', { name: 'Release' })).toBeInTheDocument();
});

test('desktop app renders a startup error when the local database cannot be opened', async () => {
  invokeMock.mockResolvedValueOnce({
    status: 'failed',
    error: {
      kind: 'database_migration',
      message: 'kukuri could not open the local app database.',
      detail: 'migration checksum mismatch',
      db_path: 'C:\\Users\\tester\\AppData\\Roaming\\kukuri\\kukuri.db',
    },
  });

  render(<App />);

  expect(await screen.findByText('kukuri could not open the local database.')).toBeInTheDocument();
  expect(screen.getByText('Migration failure')).toBeInTheDocument();
  expect(screen.getByDisplayValue(/migration checksum mismatch/)).toBeInTheDocument();
  expect(screen.queryByRole('button', { name: 'Publish' })).not.toBeInTheDocument();
});
