import { fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';

import i18n from '@/i18n';
import { createDesktopShellStore, DesktopShellStoreContext } from '@/shell/store';
import { appUpdateStore, INITIAL_UPDATE_STATE } from '@/shell/useAppUpdateStore';

import { ReleasePanel } from './ReleasePanel';

const initialStore = appUpdateStore.getState();
const check = vi.fn(async () => undefined);
const restart = vi.fn(async () => undefined);
const download = vi.fn(async () => undefined);

beforeEach(async () => {
  await i18n.changeLanguage('en');
  vi.clearAllMocks();
  appUpdateStore.setState({
    ...initialStore,
    updateState: { ...INITIAL_UPDATE_STATE },
    pendingUpdate: null,
    checkForUpdate: check,
    downloadUpdate: download,
    restartAndInstall: restart,
  });
});

afterEach(async () => {
  appUpdateStore.setState(initialStore);
  await i18n.changeLanguage('en');
});

function renderPanel() {
  return render(
    <DesktopShellStoreContext.Provider value={createDesktopShellStore()}>
      <ReleasePanel showDiagnostics={false} />
    </DesktopShellStoreContext.Provider>
  );
}

test('a verified update keeps its apply action after deferring and reopening the panel', () => {
  appUpdateStore.setState({
    updateState: { ...INITIAL_UPDATE_STATE, status: 'ready_to_restart', availableVersion: '0.1.9' },
    pendingUpdate: { version: '0.1.9', download, install: vi.fn() },
  });
  const first = renderPanel();
  const checkButton = screen.getByRole('button', { name: i18n.t('settings:release.update.check') });
  expect(checkButton).toBeDisabled();
  fireEvent.click(checkButton);
  fireEvent.click(screen.getByRole('button', { name: i18n.t('settings:release.update.later') }));
  expect(screen.getByRole('button', { name: i18n.t('settings:release.update.restartNow') })).toBeEnabled();
  expect(appUpdateStore.getState().updateState.status).toBe('ready_to_restart');
  expect(check).not.toHaveBeenCalled();
  expect(download).not.toHaveBeenCalled();
  expect(restart).not.toHaveBeenCalled();
  first.unmount();
  renderPanel();
  expect(screen.getByRole('button', { name: i18n.t('settings:release.update.check') })).toBeDisabled();
  fireEvent.click(screen.getAllByRole('button', { name: i18n.t('settings:release.update.restartNow') })[0]);
  expect(restart).toHaveBeenCalledTimes(1);
});

test.each([
  ['en', 'The update file was not found on the server. No update was installed. Try Check again later.'],
  ['ja', '配布先に更新ファイルが見つかりません。更新は適用されていません。時間を置いて「確認」からやり直してください。'],
  ['zh-CN', '更新服务器上未找到更新文件。更新未安装。请稍后重新点击“检查”。'],
])('a missing asset has localized recovery guidance in %s', async (locale, message) => {
  await i18n.changeLanguage(locale);
  appUpdateStore.setState({
    updateState: {
      ...INITIAL_UPDATE_STATE,
      status: 'failed',
      lastError: "'Download request failed with status: 404 Not Found'",
    },
  });
  renderPanel();
  expect(screen.getByText(message)).toBeInTheDocument();
  expect(screen.queryByText(/Download request failed/)).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole('button', { name: i18n.t('settings:release.update.check') }));
  expect(check).toHaveBeenCalledTimes(1);
  expect(download).not.toHaveBeenCalled();
  expect(restart).not.toHaveBeenCalled();
});
