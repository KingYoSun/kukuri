import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, expect, test } from 'vitest';

import { App } from '@/App';
import { DEVELOPER_MODE_STORAGE_KEY } from '@/lib/developerMode';
import { createDesktopMockApi } from '@/mocks/desktopApiMock';
import {
  getActiveColumn,
  openControlCenter,
  openSettingsSection,
  renderAtHash,
  selectWorkspace,
  setViewportWidth,
} from './DesktopShellPage.testHelpers';

beforeEach(() => {
  setViewportWidth(1024);
  window.history.replaceState(null, '', '/');
  // test/setup.ts は既存テスト向けに developer mode を有効化するため、既定 OFF の挙動をここで検証する。
  window.localStorage.setItem(DEVELOPER_MODE_STORAGE_KEY, 'false');
});

test('developer mode off hides Live/Game tabs and shell status badges by default', async () => {
  const user = userEvent.setup();
  const { container } = render(<App api={createDesktopMockApi()} />);

  const controlCenter = await openControlCenter(user);
  expect(within(controlCenter).getByRole('button', { name: 'Add Timeline Column' })).toBeInTheDocument();
  expect(within(controlCenter).queryByRole('button', { name: 'Add Live Column' })).not.toBeInTheDocument();
  expect(within(controlCenter).queryByRole('button', { name: 'Add Metaverse Column' })).not.toBeInTheDocument();
  expect(within(controlCenter).getByRole('button', { name: 'Add Messages Column' })).toBeInTheDocument();
  expect(within(controlCenter).getByRole('button', { name: 'Add Profile Column' })).toBeInTheDocument();

  expect(container.querySelector('.shell-status-badges')).toBeNull();
});

test('developer mode off falls back to timeline for live and game hash routes', async () => {
  renderAtHash('#/live?topic=kukuri%3Atopic%3Ageneral');

  await waitFor(() => {
    expect(window.location.hash).toMatch(/^#\/timeline/);
  });
  expect(getActiveColumn('Timeline')).toHaveAttribute('aria-current', 'true');
});

test('developer mode toggle reveals WIP tabs and badges and persists across reloads', async () => {
  const user = userEvent.setup();
  const { container, unmount } = render(<App api={createDesktopMockApi()} />);

  const drawer = await openSettingsSection(user, 'developer');
  await user.click(within(drawer).getByRole('checkbox', { name: 'Enable developer mode' }));

  expect(window.localStorage.getItem(DEVELOPER_MODE_STORAGE_KEY)).toBe('true');
  expect(container.querySelector('.shell-status-badges')).toBeNull();

  await user.keyboard('{Escape}');
  await waitFor(() => {
    expect(screen.queryByRole('dialog', { name: 'Settings' })).not.toBeInTheDocument();
  });
  const controlCenter = await openControlCenter(user);
  expect(within(controlCenter).getByRole('button', { name: 'Add Live Column' })).toBeInTheDocument();
  expect(within(controlCenter).getByRole('button', { name: 'Add Metaverse Column' })).toBeInTheDocument();

  // localStorage に永続化されるため、再マウント後も developer mode は有効のまま。
  unmount();
  render(<App api={createDesktopMockApi()} />);
  const restoredControlCenter = await openControlCenter(user);
  expect(within(restoredControlCenter).getByRole('button', { name: 'Add Live Column' })).toBeInTheDocument();
});

test('disabling developer mode while on Live routes back to timeline', async () => {
  window.localStorage.setItem(DEVELOPER_MODE_STORAGE_KEY, 'true');
  const user = userEvent.setup();
  render(<App api={createDesktopMockApi()} />);

  await selectWorkspace(user, 'Live');
  expect(window.location.hash).toMatch(/^#\/live/);

  const drawer = await openSettingsSection(user, 'developer');
  await user.click(within(drawer).getByRole('checkbox', { name: 'Enable developer mode' }));

  await waitFor(() => {
    expect(window.location.hash).toMatch(/^#\/timeline/);
    expect(getActiveColumn('Timeline')).toHaveAttribute('aria-current', 'true');
  });
  const controlCenter = await openControlCenter(user);
  expect(within(controlCenter).queryByRole('button', { name: 'Add Live Column' })).not.toBeInTheDocument();
});

test('developer mode off keeps ticket import while hiding connectivity diagnostics', async () => {
  const user = userEvent.setup();
  render(<App api={createDesktopMockApi()} />);

  const drawer = await openSettingsSection(user, 'connectivity');

  expect(within(drawer).getByText('Your Ticket')).toBeInTheDocument();
  expect(within(drawer).getByText('Peer Ticket')).toBeInTheDocument();
  expect(within(drawer).queryByText('Effective Peers')).not.toBeInTheDocument();
  expect(within(drawer).queryByText('Connected Peers')).not.toBeInTheDocument();
});
