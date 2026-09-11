import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, expect, test, vi } from 'vitest';

import { App } from '@/App';
import { DEVELOPER_MODE_STORAGE_KEY } from '@/lib/developerMode';
import { createDesktopMockApi } from '@/mocks/desktopApiMock';
import {
  buildNotification,
  openNotificationsInbox,
  openSettingsDrawer,
  renderAtHash,
  setViewportWidth,
} from './DesktopShellPage.testHelpers';

// #962: 通知の受信設定は「通知」section が正本で、通知カラムと空状態から到達できる。
beforeEach(() => {
  setViewportWidth(1024);
  window.history.replaceState(null, '', '/');
  window.localStorage.setItem(DEVELOPER_MODE_STORAGE_KEY, 'false');
});

test('settings expose a notifications section with the OS notification preferences', async () => {
  const user = userEvent.setup();
  render(<App api={createDesktopMockApi()} />);
  const drawer = await openSettingsDrawer(user);
  const nav = within(drawer).getByTestId('settings-section-notifications');
  expect(nav).toHaveTextContent('Notifications');
  await user.click(nav);
  expect(within(drawer).getByRole('checkbox', { name: 'Enable OS notifications' })).toBeVisible();
  expect(within(drawer).getByRole('checkbox', { name: 'Direct messages' })).toBeVisible();
  expect(within(drawer).getByRole('checkbox', { name: 'Mentions and replies' })).toBeVisible();
  expect(within(drawer).getByRole('checkbox', { name: 'Follows and reposts' })).toBeVisible();
  expect(within(drawer).getByRole('checkbox', { name: 'Quiet mode' })).toBeVisible();
  expect(within(drawer).getByRole('checkbox', { name: 'Show preview text' })).toBeVisible();
  expect(window.location.hash).toContain('settings=notifications');

  await user.click(within(drawer).getByTestId('settings-section-release'));
  expect(within(drawer).queryByRole('checkbox', { name: 'Enable OS notifications' })).not.toBeInTheDocument();
  expect(within(drawer).getByTestId('settings-section-release')).not.toHaveTextContent(/notification/i);
});

test('notification settings deep link opens the section and persists toggles through the existing key', async () => {
  const user = userEvent.setup();
  renderAtHash('#/timeline?topic=kukuri%3Atopic%3Ageneral&settings=notifications');
  const drawer = await screen.findByRole('dialog', { name: 'Settings' });
  await waitFor(() =>
    expect(within(drawer).getByTestId('settings-section-notifications')).toHaveAttribute(
      'aria-current',
      'location'
    )
  );
  const quiet = within(drawer).getByRole('checkbox', { name: 'Quiet mode' });
  expect(quiet).not.toBeChecked();
  await user.click(quiet);
  expect(quiet).toBeChecked();
  expect(JSON.parse(window.localStorage.getItem('kukuri:os-notification-settings:v1') ?? '{}')).toMatchObject({
    quietMode: true,
    enabled: false,
  });
});

test('the notifications inbox action opens the notification settings section without touching read state', async () => {
  const user = userEvent.setup();
  const api = createDesktopMockApi({
    notifications: [buildNotification({ notification_id: 'n-1', preview_text: 'hello' })],
  });
  const mutations = [vi.spyOn(api, 'markNotificationRead'), vi.spyOn(api, 'markAllNotificationsRead')];
  render(<App api={api} />);
  await openNotificationsInbox(user);
  const callsBefore = mutations.map((mutation) => mutation.mock.calls.length);
  await user.click(screen.getByRole('button', { name: 'Notification settings' }));
  const drawer = await screen.findByRole('dialog', { name: 'Settings' });
  expect(within(drawer).getByTestId('settings-section-notifications')).toHaveAttribute(
    'aria-current',
    'location'
  );
  expect(window.location.hash).toContain('settings=notifications');
  mutations.forEach((mutation, index) => expect(mutation.mock.calls.length).toBe(callsBefore[index]));
});

test('the empty notifications inbox offers the notification settings as its next action', async () => {
  const user = userEvent.setup();
  renderAtHash('#/notifications?topic=kukuri%3Atopic%3Ageneral', createDesktopMockApi());
  expect(await screen.findByText('No notifications yet.')).toBeInTheDocument();
  await user.click(screen.getByRole('button', { name: 'Notification settings' }));
  const drawer = await screen.findByRole('dialog', { name: 'Settings' });
  expect(within(drawer).getByRole('checkbox', { name: 'Enable OS notifications' })).toBeVisible();
  await user.click(within(drawer).getByRole('button', { name: 'Close settings' }));
  await waitFor(() => expect(screen.queryByRole('dialog', { name: 'Settings' })).not.toBeInTheDocument());
  expect(window.location.hash).not.toContain('settings=');
  expect(screen.getByText('No notifications yet.')).toBeInTheDocument();
});
