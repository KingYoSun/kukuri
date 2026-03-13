import { beforeEach, describe, expect, it, vi } from 'vitest';
import { act, renderHook, waitFor } from '@testing-library/react';
import { usePrivacySettingsAutoSync } from '@/hooks/usePrivacySettingsAutoSync';
import { syncPrivacySettings } from '@/lib/settings/privacySettingsSync';
import { useAuthStore } from '@/stores/authStore';
import { useOfflineStore } from '@/stores/offlineStore';
import { usePrivacySettingsStore } from '@/stores/privacySettingsStore';

vi.mock('@/stores/authStore', () => {
  const state = {
    currentUser: null as null | { npub: string },
    updateUser: vi.fn(),
  };

  const useAuthStore = ((selector?: (value: typeof state) => unknown) =>
    selector ? selector(state) : state) as unknown as {
    (selector?: (value: typeof state) => unknown): unknown;
    getState: () => typeof state;
    setState: (
      updater: Partial<typeof state> | ((current: typeof state) => Partial<typeof state>),
    ) => void;
  };

  useAuthStore.getState = () => state;
  useAuthStore.setState = (updater) => {
    const next = typeof updater === 'function' ? updater(state) : updater;
    Object.assign(state, next);
  };

  return { useAuthStore };
});

vi.mock('@/stores/offlineStore', () => {
  const state = {
    isOnline: false,
  };

  const useOfflineStore = ((selector?: (value: typeof state) => unknown) =>
    selector ? selector(state) : state) as unknown as {
    (selector?: (value: typeof state) => unknown): unknown;
    getState: () => typeof state;
    setState: (
      updater: Partial<typeof state> | ((current: typeof state) => Partial<typeof state>),
    ) => void;
  };

  useOfflineStore.getState = () => state;
  useOfflineStore.setState = (updater) => {
    const next = typeof updater === 'function' ? updater(state) : updater;
    Object.assign(state, next);
  };

  return { useOfflineStore };
});

vi.mock('@/stores/privacySettingsStore', () => {
  const store = {
    publicProfile: true,
    showOnlineStatus: false,
    hasPendingSync: false,
    lastSyncError: null as string | null,
    markSyncSuccess: vi.fn(() => {
      store.hasPendingSync = false;
      store.lastSyncError = null;
    }),
    markSyncFailure: vi.fn((reason?: string | null) => {
      store.hasPendingSync = true;
      store.lastSyncError = reason ?? 'sync_failed';
    }),
  };

  const usePrivacySettingsStore = ((selector?: (value: typeof store) => unknown) =>
    selector ? selector(store) : store) as unknown as {
    (selector?: (value: typeof store) => unknown): unknown;
    getState: () => typeof store;
    setState: (
      updater: Partial<typeof store> | ((current: typeof store) => Partial<typeof store>),
    ) => void;
  };

  usePrivacySettingsStore.getState = () => store;
  usePrivacySettingsStore.setState = (updater) => {
    const next = typeof updater === 'function' ? updater(store) : updater;
    Object.assign(store, next);
  };

  return { usePrivacySettingsStore };
});

vi.mock('@/lib/settings/privacySettingsSync', () => ({
  syncPrivacySettings: vi.fn(),
}));

vi.mock('@/lib/errorHandler', () => ({
  errorHandler: {
    log: vi.fn(),
  },
}));

describe('usePrivacySettingsAutoSync', () => {
  beforeEach(() => {
    vi.clearAllMocks();

    useAuthStore.setState({
      currentUser: null,
      updateUser: vi.fn(),
    });
    useOfflineStore.setState({ isOnline: false });
    usePrivacySettingsStore.setState({
      publicProfile: true,
      showOnlineStatus: false,
      hasPendingSync: false,
      lastSyncError: null,
    });
  });

  it('online復帰イベントで未同期プライバシー設定を同期する', async () => {
    useAuthStore.setState({
      currentUser: { npub: 'npub1alice' },
    });
    useOfflineStore.setState({ isOnline: false });
    usePrivacySettingsStore.setState({
      publicProfile: false,
      showOnlineStatus: true,
      hasPendingSync: true,
    });
    vi.mocked(syncPrivacySettings).mockResolvedValue(undefined);

    renderHook(() => usePrivacySettingsAutoSync());

    act(() => {
      useOfflineStore.setState({ isOnline: true });
      window.dispatchEvent(new Event('online'));
    });

    await waitFor(() =>
      expect(syncPrivacySettings).toHaveBeenCalledWith({
        npub: 'npub1alice',
        publicProfile: false,
        showOnlineStatus: true,
      }),
    );
    expect(usePrivacySettingsStore.getState().markSyncSuccess).toHaveBeenCalledTimes(1);
    expect(useAuthStore.getState().updateUser).toHaveBeenCalledWith({
      publicProfile: false,
      showOnlineStatus: true,
    });
  });

  it('ユーザー復元時に未同期設定があれば同期する', async () => {
    useOfflineStore.setState({ isOnline: true });
    usePrivacySettingsStore.setState({
      publicProfile: false,
      showOnlineStatus: false,
      hasPendingSync: true,
    });
    vi.mocked(syncPrivacySettings).mockResolvedValue(undefined);

    const { rerender } = renderHook(() => usePrivacySettingsAutoSync());

    expect(syncPrivacySettings).not.toHaveBeenCalled();

    act(() => {
      useAuthStore.setState({
        currentUser: { npub: 'npub1alice' },
      });
      rerender();
    });

    await waitFor(() =>
      expect(syncPrivacySettings).toHaveBeenCalledWith({
        npub: 'npub1alice',
        publicProfile: false,
        showOnlineStatus: false,
      }),
    );
  });
});
