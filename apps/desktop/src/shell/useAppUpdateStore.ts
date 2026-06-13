import { useStore } from 'zustand';
import { createStore } from 'zustand/vanilla';

import packageJson from '../../package.json';

import { isTauriRuntime, type UpdateState } from '@/lib/releaseReadiness';

export type PendingUpdate = {
  version: string;
  downloadAndInstall: (onEvent?: (event: unknown) => void) => Promise<void>;
};

export const INITIAL_UPDATE_STATE: UpdateState = {
  status: 'idle',
  currentVersion: packageJson.version,
  availableVersion: null,
  downloadedBytes: 0,
  contentLength: null,
  lastError: null,
};

function updateStateFromError(currentVersion: string, error: unknown): UpdateState {
  return {
    status: 'failed',
    currentVersion,
    availableVersion: null,
    lastError: error instanceof Error ? error.message : String(error),
  };
}

export type AppUpdateStore = {
  updateState: UpdateState;
  pendingUpdate: PendingUpdate | null;
  checkForUpdate: () => Promise<void>;
  installUpdate: () => Promise<void>;
};

export const appUpdateStore = createStore<AppUpdateStore>((set, get) => ({
  updateState: INITIAL_UPDATE_STATE,
  pendingUpdate: null,
  checkForUpdate: async () => {
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
        import('@tauri-apps/plugin-updater'),
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
        },
      });
    } catch (error) {
      set((state) => ({
        updateState: updateStateFromError(state.updateState.currentVersion, error),
      }));
    }
  },
  installUpdate: async () => {
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
      await pendingUpdate.downloadAndInstall((event) => {
        if (!event || typeof event !== 'object' || !('event' in event)) {
          return;
        }
        const downloadEvent = event as {
          event: string;
          data?: { chunkLength?: number; contentLength?: number };
        };
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
        updateState: updateStateFromError(state.updateState.currentVersion, error),
      }));
    }
  },
}));

export const selectUpdateAvailable = (state: AppUpdateStore): boolean =>
  state.updateState.status === 'available';

export function useAppUpdateStore<T>(selector: (state: AppUpdateStore) => T): T {
  return useStore(appUpdateStore, selector);
}
