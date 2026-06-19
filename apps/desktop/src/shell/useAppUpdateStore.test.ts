import { beforeEach, describe, expect, test, vi } from 'vitest';
import type { DownloadEvent } from '@tauri-apps/plugin-updater';

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
  });
});
