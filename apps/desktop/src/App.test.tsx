import { render, screen, waitFor, within } from '@testing-library/react';
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
  const timelineColumn = screen.getByRole('region', { name: /^Timeline Column/ });
  expect(timelineColumn).toHaveTextContent('general');
  expect(within(timelineColumn).getByRole('button', { name: /^Publish to / })).toBeInTheDocument();
  expect(screen.getByTestId('control-center-trigger')).toBeInTheDocument();
  expect(window.localStorage.getItem(DESKTOP_THEME_STORAGE_KEY)).toBe('dark');
});

test('desktop app restores a persisted light theme on boot', async () => {
  window.localStorage.setItem(DESKTOP_THEME_STORAGE_KEY, 'light');

  render(<App api={createDesktopMockApi()} />);

  await waitFor(() => {
    expect(document.documentElement).toHaveAttribute('data-theme', 'light');
  });
});

test('settings drawer can open the release section', async () => {
  const user = userEvent.setup();
  render(<App api={createDesktopMockApi()} />);

  await user.click(await screen.findByTestId('control-center-trigger'));
  await user.click(
    within(screen.getByRole('complementary', { name: 'Control Center' })).getByRole('button', {
      name: 'Settings',
    })
  );
  expect(screen.getByRole('dialog', { name: 'Settings' })).toBeInTheDocument();

  await user.click(screen.getByRole('button', { name: 'Release' }));

  expect(screen.getByRole('heading', { name: 'Release' })).toBeInTheDocument();
  expect(screen.getByRole('link', { name: /Latest release/ })).toHaveAttribute(
    'href',
    'https://github.com/KingYoSun/kukuri/releases/latest'
  );
  expect(screen.getByRole('link', { name: /Privacy policy/ })).toHaveAttribute(
    'href',
    'https://api.kukuri.app/privacy'
  );
});

test('desktop app blocks startup until app-level legal consent is accepted', async () => {
  const user = userEvent.setup();
  invokeMock.mockResolvedValueOnce({
    status: 'consent_required',
    current_bundle_version: 2,
    accepted_bundle_version: null,
  });
  invokeMock.mockResolvedValueOnce({
    status: 'failed',
    error: {
      kind: 'unknown',
      message: 'kukuri could not open the local app database.',
      detail: 'runtime starts after consent',
      db_path: null,
    },
  });

  render(<App />);

  expect(await screen.findByRole('heading', { name: 'Before you continue' })).toBeInTheDocument();
  expect(screen.getByText('Terms of Service')).toBeInTheDocument();
  expect(screen.getByText('Privacy Policy')).toBeInTheDocument();
  expect(
    screen.queryByText('These documents are drafts and are not legal advice.')
  ).not.toBeInTheDocument();
  expect(
    screen.queryByText(
      'This is a draft and is not legal advice. Final decisions should be made in consultation with appropriate experts or regulators.'
    )
  ).not.toBeInTheDocument();
  expect(
    screen.queryByText(
      'The terms of service or privacy policy have been updated. Please review and accept again to continue.'
    )
  ).not.toBeInTheDocument();
  expect(screen.queryByRole('button', { name: 'Publish' })).not.toBeInTheDocument();

  await user.click(screen.getByRole('button', { name: 'Accept and continue' }));

  await waitFor(() => {
    expect(invokeMock).toHaveBeenCalledWith('accept_app_consents', { bundleVersion: 2 });
  });
  expect(await screen.findByText('kukuri could not open the local database.')).toBeInTheDocument();
  expect(screen.getByDisplayValue(/runtime starts after consent/)).toBeInTheDocument();
});

test('desktop app requires renewed consent for an older legal bundle', async () => {
  const user = userEvent.setup();
  invokeMock.mockResolvedValueOnce({
    status: 'consent_required',
    current_bundle_version: 2,
    accepted_bundle_version: 1,
  });
  invokeMock.mockResolvedValueOnce({
    status: 'failed',
    error: {
      kind: 'unknown',
      message: 'kukuri could not open the local app database.',
      detail: 'runtime starts only after renewed consent',
      db_path: null,
    },
  });

  render(<App />);

  expect(
    await screen.findAllByText(
      'The terms of service or privacy policy have been updated. Please review and accept again to continue.'
    )
  ).toHaveLength(1);
  expect(
    screen.queryByText('These documents are drafts and are not legal advice.')
  ).not.toBeInTheDocument();
  expect(
    screen.queryByText(
      'This is a draft and is not legal advice. Final decisions should be made in consultation with appropriate experts or regulators.'
    )
  ).not.toBeInTheDocument();
  expect(screen.getByText('v2')).toBeInTheDocument();
  expect(screen.queryByTestId('control-center-trigger')).not.toBeInTheDocument();

  await user.click(screen.getByRole('button', { name: 'Accept and continue' }));

  await waitFor(() => {
    expect(invokeMock).toHaveBeenCalledWith('accept_app_consents', { bundleVersion: 2 });
  });
  expect(await screen.findByText('kukuri could not open the local database.')).toBeInTheDocument();
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
