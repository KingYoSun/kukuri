import { render, screen, waitFor } from '@testing-library/react';
import { beforeEach, expect, test } from 'vitest';

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
