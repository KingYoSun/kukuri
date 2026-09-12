import { useStore } from 'zustand';
import { createStore } from 'zustand/vanilla';
import type { DownloadEvent } from '@tauri-apps/plugin-updater';
import { invoke } from '@tauri-apps/api/core';

import packageJson from '../../package.json';

import { isTauriRuntime, type UpdateState } from '@/lib/releaseReadiness';

export type PendingUpdate = {
  version: string;
  download: (onEvent?: (event: DownloadEvent) => void) => Promise<void>;
  install: () => Promise<void>;
};

export const INITIAL_UPDATE_STATE: UpdateState = {
  status: 'idle',
  currentVersion: packageJson.version,
  availableVersion: null,
  downloadedBytes: 0,
  contentLength: null,
  lastError: null,
  lastCheckedAt: null,
};

function updateStateFromError(currentVersion: string, error: unknown): UpdateState {
  return {
    status: 'failed',
    currentVersion,
    availableVersion: null,
    lastError: error instanceof Error ? error.message : String(error),
  };
}

function updateFailureFromError(state: UpdateState, error: unknown): UpdateState {
  return {
    ...updateStateFromError(state.currentVersion, error),
    lastCheckedAt: state.lastCheckedAt ?? null,
  };
}

export type AppUpdateStore = {
  updateState: UpdateState;
  pendingUpdate: PendingUpdate | null;
  checkForUpdate: () => Promise<void>;
  downloadUpdate: () => Promise<void>;
  restartAndInstall: () => Promise<void>;
};

export const appUpdateStore = createStore<AppUpdateStore>((set, get) => ({
  updateState: INITIAL_UPDATE_STATE,
  pendingUpdate: null,
  checkForUpdate: async () => {
    if (['checking', 'downloading', 'ready_to_restart', 'installing'].includes(get().updateState.status)) return;
    set((state) => ({
      updateState: {
        ...state.updateState,
        status: 'checking',
        lastError: null,
      },
    }));
    try {
      const [{ getVersion }, updater] = await Promise.all([
        import('@tauri-apps/api/app'),
        import('@/lib/appUpdater'),
      ]);
      const currentVersion = isTauriRuntime() ? await getVersion() : packageJson.version;
      const update = isTauriRuntime() ? await updater.check() : null;
      if (!update) {
        set({
          pendingUpdate: null,
          updateState: {
            status: 'up_to_date',
            currentVersion,
            availableVersion: null,
            lastError: null,
            lastCheckedAt: Date.now(),
          },
        });
        return;
      }
      set({
        pendingUpdate: update,
        updateState: {
          status: 'available',
          currentVersion,
          availableVersion: update.version,
          lastError: null,
          lastCheckedAt: Date.now(),
        },
      });
    } catch (error) {
      set((state) => ({
        updateState: {
          ...updateStateFromError(state.updateState.currentVersion, error),
          lastCheckedAt: Date.now(),
        },
      }));
    }
  },
  downloadUpdate: async () => {
    if (['checking', 'downloading', 'ready_to_restart', 'installing'].includes(get().updateState.status)) return;
    const { pendingUpdate, checkForUpdate } = get();
    if (!pendingUpdate) {
      await checkForUpdate();
      return;
    }
    set((state) => ({
      updateState: {
        ...state.updateState,
        status: 'downloading',
        downloadedBytes: 0,
        contentLength: null,
        lastError: null,
      },
    }));
    try {
      await pendingUpdate.download((downloadEvent) => {
        set((state) => {
          if (downloadEvent.event === 'Started') {
            return {
              updateState: {
                ...state.updateState,
                contentLength: downloadEvent.data?.contentLength ?? null,
                downloadedBytes: 0,
              },
            };
          }
          if (downloadEvent.event === 'Progress') {
            return {
              updateState: {
                ...state.updateState,
                downloadedBytes:
                  (state.updateState.downloadedBytes ?? 0) + (downloadEvent.data?.chunkLength ?? 0),
              },
            };
          }
          return state;
        });
      });
      set((state) => ({
        updateState: {
          ...state.updateState,
          status: 'ready_to_restart',
          lastError: null,
        },
      }));
    } catch (error) {
      set((state) => ({
        updateState: updateFailureFromError(state.updateState, error),
      }));
    }
  },
  restartAndInstall: async () => {
    const { pendingUpdate, updateState } = get();
    if (!pendingUpdate || updateState.status !== 'ready_to_restart') {
      return;
    }
    set((state) => ({ updateState: { ...state.updateState, status: 'installing' } }));
    try {
      await pendingUpdate.install();
      // Linuxではinstallだけでは終了しない。backend側でruntimeを停止してから再起動する。
      await invoke('restart_after_update');
    } catch (error) {
      set((state) => ({
        updateState: updateFailureFromError(state.updateState, error),
      }));
    }
  },
}));

export const selectUpdateAvailable = (state: AppUpdateStore): boolean =>
  state.updateState.status === 'available';

export function useAppUpdateStore<T>(selector: (state: AppUpdateStore) => T): T {
  return useStore(appUpdateStore, selector);
}
