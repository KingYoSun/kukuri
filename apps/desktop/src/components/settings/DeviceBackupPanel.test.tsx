import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';

import { DESKTOP_THEME_STORAGE_KEY } from '@/lib/theme';

import { DeviceBackupPanel } from './DeviceBackupPanel';

const backupApi = vi.hoisted(() => ({
  applyPortableFrontendState: vi.fn(),
  cancelDeviceBackup: vi.fn(),
  capturePortableFrontendState: vi.fn(),
  chooseDeviceBackupDestination: vi.fn(),
  chooseDeviceBackupSource: vi.fn(),
  createDeviceBackup: vi.fn(),
  listenDeviceBackupProgress: vi.fn(),
  previewDeviceBackup: vi.fn(),
  restoreDeviceBackup: vi.fn(),
}));

vi.mock('@/lib/api/deviceBackup', () => backupApi);

beforeEach(() => {
  backupApi.listenDeviceBackupProgress.mockResolvedValue(() => {});
  backupApi.capturePortableFrontendState.mockReturnValue({
    [DESKTOP_THEME_STORAGE_KEY]: 'dark',
  });
  backupApi.chooseDeviceBackupDestination.mockResolvedValue('C:\\backup.kukuri-backup');
  backupApi.createDeviceBackup.mockResolvedValue({
    path: 'C:\\backup.kukuri-backup',
    public_key: 'aa'.repeat(32),
    bytes: 4096,
  });
});

afterEach(() => {
  vi.clearAllMocks();
});

test('creates one encrypted backup file only after risk acknowledgement', async () => {
  const user = userEvent.setup();
  render(<DeviceBackupPanel />);

  const submit = screen.getByRole('button', { name: 'Choose destination and create' });
  expect(submit).toBeDisabled();
  await user.click(screen.getByTestId('device-backup-acknowledge'));
  await user.type(screen.getByTestId('device-backup-passphrase'), 'long passphrase');
  await user.type(screen.getByTestId('device-backup-passphrase-confirm'), 'long passphrase');
  expect(submit).toBeEnabled();
  await user.click(submit);

  await waitFor(() => {
    expect(backupApi.createDeviceBackup).toHaveBeenCalledWith(
      'C:\\backup.kukuri-backup',
      'long passphrase',
      { [DESKTOP_THEME_STORAGE_KEY]: 'dark' }
    );
  });
  expect(await screen.findByTestId('device-backup-created')).toHaveTextContent('4.0 KiB');
});

test('requires explicit confirmation before replacing the same account', async () => {
  const user = userEvent.setup();
  backupApi.chooseDeviceBackupSource.mockResolvedValue('C:\\backup.kukuri-backup');
  backupApi.previewDeviceBackup.mockResolvedValue({
    public_key: 'aa'.repeat(32),
    account_label: null,
    created_at: 1,
    app_version: '0.1.8',
    content_bytes: 4096,
    existing_account_id: 'aa'.repeat(8),
    included: ['account_key'],
    requires_reconsent: ['app_legal_documents'],
  });
  render(<DeviceBackupPanel />);

  await user.click(screen.getByRole('button', { name: 'Choose backup file' }));
  await user.type(screen.getByTestId('device-restore-passphrase'), 'long passphrase');
  await user.click(screen.getByRole('button', { name: 'Review contents' }));

  const restore = await screen.findByRole('button', { name: 'Validate and restore' });
  expect(restore).toBeDisabled();
  await user.click(screen.getByTestId('device-restore-replace-confirm'));
  expect(restore).toBeEnabled();
});
