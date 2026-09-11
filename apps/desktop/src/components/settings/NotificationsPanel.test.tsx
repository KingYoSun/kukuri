import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';

import i18n from '@/i18n';
import {
  getOsNotificationPermission,
  requestOsNotificationPermission,
} from '@/lib/api/osNotificationPermission';
import { saveOsNotificationSettings } from '@/lib/releaseReadiness';
import { createDesktopShellStore, DesktopShellStoreContext } from '@/shell/store';

import { NotificationsPanel } from './NotificationsPanel';

vi.mock('@/lib/api/osNotificationPermission', () => ({
  getOsNotificationPermission: vi.fn(),
  requestOsNotificationPermission: vi.fn(),
}));
vi.mock('@/lib/releaseReadiness', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/lib/releaseReadiness')>()),
  isTauriRuntime: () => true,
  saveOsNotificationSettings: vi.fn(),
}));

beforeEach(() => {
  vi.mocked(getOsNotificationPermission).mockResolvedValue('granted');
  vi.mocked(requestOsNotificationPermission).mockResolvedValue('granted');
});
afterEach(() => vi.resetAllMocks());

function renderPanel() {
  return render(
    <DesktopShellStoreContext.Provider value={createDesktopShellStore()}>
      <NotificationsPanel />
    </DesktopShellStoreContext.Provider>
  );
}

test('initial query failure is unavailable, not an unrequested permission', async () => {
  vi.mocked(getOsNotificationPermission).mockRejectedValue(new Error('native unavailable'));
  renderPanel();
  expect(await screen.findByText(/Unavailable/i)).toBeInTheDocument();
  expect(saveOsNotificationSettings).not.toHaveBeenCalled();
});

test('Linux service availability is not presented as permission granted', async () => {
  vi.mocked(getOsNotificationPermission).mockResolvedValue('available');
  renderPanel();
  expect(await screen.findByText(/OS settings/i)).toBeInTheDocument();
  expect(saveOsNotificationSettings).not.toHaveBeenCalled();
});

test('failed explicit check is contained and does not enable notifications', async () => {
  vi.mocked(requestOsNotificationPermission).mockRejectedValue(new Error('native unavailable'));
  renderPanel();
  await screen.findByText(/Granted/i);
  fireEvent.click(screen.getByRole('button', { name: /permission|availability/i }));
  expect(await screen.findByText(/Unavailable/i)).toBeInTheDocument();
  expect(saveOsNotificationSettings).not.toHaveBeenCalled();
});

test('checking keeps the previous result and prevents duplicate requests', async () => {
  let resolveRequest!: (value: string) => void;
  const request = new Promise<string>((resolve) => { resolveRequest = resolve; });
  vi.mocked(requestOsNotificationPermission).mockReturnValue(request);
  renderPanel();
  await screen.findByText(/Granted/i);
  const button = screen.getByRole('button', { name: /permission|availability/i });
  fireEvent.click(button);
  fireEvent.click(button);
  expect(requestOsNotificationPermission).toHaveBeenCalledTimes(1);
  expect(button).toBeDisabled();
  expect(screen.getByText(/Granted/i)).toBeInTheDocument();
  resolveRequest('available');
  await waitFor(() => expect(button).toBeEnabled());
  expect(await screen.findByText(/OS settings/i)).toBeInTheDocument();
  expect(saveOsNotificationSettings).not.toHaveBeenCalled();
});

test.each(['en', 'ja', 'zh-CN'])('service status and recheck are localized in %s', async (locale) => {
  await i18n.changeLanguage(locale);
  vi.mocked(getOsNotificationPermission).mockResolvedValue('available');
  renderPanel();
  const status = i18n.t('settings:notifications.osNotifications.permissions.available');
  expect(await screen.findByText(i18n.t('settings:notifications.osNotifications.permission', { permission: status })))
    .toBeInTheDocument();
  expect(screen.getByRole('button', { name: i18n.t('settings:notifications.osNotifications.requestPermission') }))
    .toBeEnabled();
});

test('explicit Windows granted result keeps the existing enable action', async () => {
  renderPanel();
  await screen.findByText(/Granted/i);
  fireEvent.click(screen.getByRole('button', { name: /permission|availability/i }));
  await waitFor(() =>
    expect(saveOsNotificationSettings).toHaveBeenCalledWith(
      expect.objectContaining({ enabled: true, previewBody: false })
    )
  );
});
