import { beforeEach, describe, expect, test, vi } from 'vitest';
import type { DownloadEvent } from '@tauri-apps/plugin-updater';

const { invoke, updaterCheck, getVersion } = vi.hoisted(() => ({
  invoke: vi.fn(async () => undefined),
  updaterCheck: vi.fn(),
  getVersion: vi.fn(async () => '0.1.8'),
}));
vi.mock('@tauri-apps/api/core', () => ({ invoke }));
vi.mock('@tauri-apps/api/app', () => ({ getVersion }));
vi.mock('@/lib/appUpdater', () => ({ check: updaterCheck }));
vi.mock('@/lib/releaseReadiness', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/lib/releaseReadiness')>()),
  isTauriRuntime: () => true,
}));

import {
  appUpdateStore,
  INITIAL_UPDATE_STATE,
  type PendingUpdate,
} from './useAppUpdateStore';

function resetUpdateStore(): void {
  appUpdateStore.setState({
    updateState: { ...INITIAL_UPDATE_STATE },
    pendingUpdate: null,
  });
}

function pendingUpdate(overrides: Partial<PendingUpdate> = {}): PendingUpdate {
  return {
    version: '0.1.3',
    download: vi.fn(async () => undefined),
    install: vi.fn(async () => undefined),
    ...overrides,
  };
}

describe('app update store', () => {
  beforeEach(() => {
    resetUpdateStore();
    vi.clearAllMocks();
    updaterCheck.mockReset().mockResolvedValue(null);
  });

  test('downloadUpdate downloads the pending update and waits for restart', async () => {
    const download = vi.fn(async (onEvent?: (event: DownloadEvent) => void) => {
      onEvent?.({ event: 'Started', data: { contentLength: 12 } });
      onEvent?.({ event: 'Progress', data: { chunkLength: 5 } });
    });
    const update = pendingUpdate({ download });
    appUpdateStore.setState({
      pendingUpdate: update,
      updateState: {
        ...INITIAL_UPDATE_STATE,
        status: 'available',
        availableVersion: update.version,
      },
    });

    await appUpdateStore.getState().downloadUpdate();

    expect(download).toHaveBeenCalledTimes(1);
    expect(appUpdateStore.getState().updateState).toMatchObject({
      status: 'ready_to_restart',
      availableVersion: update.version,
      downloadedBytes: 5,
      contentLength: 12,
      lastError: null,
    });
  });

  test('restartAndInstall installs a downloaded pending update', async () => {
    const install = vi.fn(async () => undefined);
    const update = pendingUpdate({ install });
    appUpdateStore.setState({
      pendingUpdate: update,
      updateState: {
        ...INITIAL_UPDATE_STATE,
        status: 'ready_to_restart',
        availableVersion: update.version,
      },
    });

    await appUpdateStore.getState().restartAndInstall();

    expect(install).toHaveBeenCalledTimes(1);
    expect(invoke).toHaveBeenCalledWith('restart_after_update');
    expect(install.mock.invocationCallOrder[0]).toBeLessThan(invoke.mock.invocationCallOrder[0]);
  });

  test.each(['available', 'up_to_date', 'unreachable'])(
    'rechecks preserve the verified update even when the server would be %s',
    async (response) => {
      const update = pendingUpdate();
      if (response === 'available') updaterCheck.mockResolvedValue(pendingUpdate({ version: '0.1.9' }));
      if (response === 'unreachable') updaterCheck.mockRejectedValue(new Error('network unavailable'));
      appUpdateStore.setState({
        pendingUpdate: update,
        updateState: { ...INITIAL_UPDATE_STATE, status: 'available', availableVersion: update.version },
      });
      await appUpdateStore.getState().downloadUpdate();
      const verifiedState = appUpdateStore.getState().updateState;

      await appUpdateStore.getState().checkForUpdate();
      await appUpdateStore.getState().checkForUpdate();

      expect(updaterCheck).not.toHaveBeenCalled();
      expect(getVersion).not.toHaveBeenCalled();
      expect(appUpdateStore.getState().pendingUpdate).toBe(update);
      expect(appUpdateStore.getState().updateState).toEqual(verifiedState);
      expect(update.download).toHaveBeenCalledTimes(1);
      expect(update.install).not.toHaveBeenCalled();
      expect(invoke).not.toHaveBeenCalled();
      await appUpdateStore.getState().restartAndInstall();
      expect(update.install).toHaveBeenCalledTimes(1);
      expect(invoke).toHaveBeenCalledTimes(1);
    }
  );

  // #956 Reopen: 確認の完了時刻はstoreが所有し、結果が同じでも更新される（AC-6 / INVAR-1）。
  test.each(['up_to_date', 'available', 'failed'])(
    'checkForUpdate records when a %s check completed',
    async (outcome) => {
      vi.useFakeTimers({ toFake: ['Date'] });
      if (outcome === 'available') updaterCheck.mockResolvedValue(pendingUpdate({ version: '0.1.9' }));
      if (outcome === 'failed') updaterCheck.mockRejectedValue(new Error('network unavailable'));
      expect(INITIAL_UPDATE_STATE.lastCheckedAt).toBeNull();
      const first = Date.UTC(2026, 8, 12, 3, 4, 0);
      vi.setSystemTime(first);
      const started = appUpdateStore.getState().checkForUpdate();
      expect(appUpdateStore.getState().updateState.lastCheckedAt).toBeNull();
      await started;
      expect(appUpdateStore.getState().updateState).toMatchObject({ status: outcome, lastCheckedAt: first });
      const second = first + 5 * 60_000;
      vi.setSystemTime(second);
      await appUpdateStore.getState().checkForUpdate();
      expect(appUpdateStore.getState().updateState).toMatchObject({ status: outcome, lastCheckedAt: second });
      expect(updaterCheck).toHaveBeenCalledTimes(2);
      expect(invoke).not.toHaveBeenCalled();
    }
  );

  test('a repeated download cannot discard a verified update', async () => {
    const update = pendingUpdate();
    appUpdateStore.setState({
      pendingUpdate: update,
      updateState: { ...INITIAL_UPDATE_STATE, status: 'available', availableVersion: update.version },
    });
    await appUpdateStore.getState().downloadUpdate();
    const verifiedState = appUpdateStore.getState().updateState;
    await appUpdateStore.getState().downloadUpdate();
    expect(update.download).toHaveBeenCalledTimes(1);
    expect(appUpdateStore.getState().updateState).toEqual(verifiedState);
    expect(update.install).not.toHaveBeenCalled();
    expect(invoke).not.toHaveBeenCalled();
  });

  test('failed install never requests a restart', async () => {
    const install = vi.fn(async () => { throw new Error('replace failed'); });
    appUpdateStore.setState({
      pendingUpdate: pendingUpdate({ install }),
      updateState: { ...INITIAL_UPDATE_STATE, status: 'ready_to_restart' },
    });
    await appUpdateStore.getState().restartAndInstall();
    expect(invoke).not.toHaveBeenCalled();
    expect(appUpdateStore.getState().updateState.lastError).toBe('replace failed');
  });

  test('concurrent activation only installs and restarts once', async () => {
    let finish!: () => void;
    const install = vi.fn(() => new Promise<void>((resolve) => { finish = resolve; }));
    appUpdateStore.setState({
      pendingUpdate: pendingUpdate({ install }),
      updateState: { ...INITIAL_UPDATE_STATE, status: 'ready_to_restart' },
    });
    const first = appUpdateStore.getState().restartAndInstall();
    expect(appUpdateStore.getState().updateState.status).toBe('installing');
    await appUpdateStore.getState().restartAndInstall();
    expect(install).toHaveBeenCalledTimes(1);
    expect(invoke).not.toHaveBeenCalled();
    finish();
    await first;
    expect(invoke).toHaveBeenCalledTimes(1);
  });

  test('downloadUpdate records download failures', async () => {
    const download = vi.fn(async () => {
      throw new Error('download failed');
    });
    const update = pendingUpdate({ download });
    appUpdateStore.setState({
      pendingUpdate: update,
      updateState: {
        ...INITIAL_UPDATE_STATE,
        status: 'available',
        availableVersion: update.version,
      },
    });

    await appUpdateStore.getState().downloadUpdate();

    expect(appUpdateStore.getState().updateState).toMatchObject({
      status: 'failed',
      lastError: 'download failed',
    });
  });

  test('a rejected updater signature can never reach install', async () => {
    const download = vi.fn(async () => {
      throw new Error('signature verification failed for updater bundle');
    });
    const install = vi.fn(async () => undefined);
    const update = pendingUpdate({ download, install });
    appUpdateStore.setState({
      pendingUpdate: update,
      updateState: {
        ...INITIAL_UPDATE_STATE,
        status: 'available',
        availableVersion: update.version,
      },
    });

    await appUpdateStore.getState().downloadUpdate();
    await appUpdateStore.getState().restartAndInstall();

    expect(download).toHaveBeenCalledTimes(1);
    expect(appUpdateStore.getState().updateState).toMatchObject({
      status: 'failed',
      lastError: 'signature verification failed for updater bundle',
    });
    expect(install).not.toHaveBeenCalled();
    expect(invoke).not.toHaveBeenCalled();
  });

  test('restartAndInstall is ignored before an update is ready to restart', async () => {
    const install = vi.fn(async () => undefined);
    const update = pendingUpdate({ install });
    appUpdateStore.setState({
      pendingUpdate: update,
      updateState: {
        ...INITIAL_UPDATE_STATE,
        status: 'available',
        availableVersion: update.version,
      },
    });

    await appUpdateStore.getState().restartAndInstall();

    expect(install).not.toHaveBeenCalled();
    expect(invoke).not.toHaveBeenCalled();
  });
});
