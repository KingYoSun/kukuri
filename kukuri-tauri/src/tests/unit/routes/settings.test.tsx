import { beforeEach, describe, expect, it, vi } from 'vitest';
import { act, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { SettingsPage } from '@/routes/settings';
import { useUIStore } from '@/stores/uiStore';
import { useOfflineStore } from '@/stores/offlineStore';
import { usePrivacySettingsStore } from '@/stores/privacySettingsStore';
import { useAuthStore } from '@/stores/authStore';
import { TauriApi } from '@/lib/api/tauri';
import { updateNostrMetadata } from '@/lib/api/nostr';
import i18n from '@/i18n';

vi.mock('@/components/NostrTestPanel', () => ({
  NostrTestPanel: () => <div>Nostr Panel</div>,
}));

vi.mock('@/components/P2PDebugPanel', () => ({
  P2PDebugPanel: () => <div>P2P Debug Panel</div>,
}));

vi.mock('@/components/p2p/PeerConnectionPanel', () => ({
  PeerConnectionPanel: () => <div>Peer Connection Panel</div>,
}));

vi.mock('@/components/p2p/BootstrapConfigPanel', () => ({
  BootstrapConfigPanel: () => <div>Bootstrap Panel</div>,
}));

vi.mock('@/stores/authStore', () => {
  const listeners = new Set<(state: any, previousState: any) => void>();
  const state = {
    currentUser: null as null | {
      id: string;
      pubkey: string;
      npub: string;
      name: string;
      displayName: string;
      about: string;
      picture: string;
      nip05: string;
      publicProfile: boolean;
      showOnlineStatus: boolean;
    },
    updateUser: vi.fn(),
  };
  const useAuthStore = vi.fn(() => state) as unknown as {
    (): typeof state;
    getState: () => typeof state;
    setState: (
      updater: Partial<typeof state> | ((current: typeof state) => Partial<typeof state>),
    ) => void;
    subscribe: (listener: (state: typeof state, previousState: typeof state) => void) => () => void;
  };
  useAuthStore.getState = () => state;
  useAuthStore.setState = (updater) => {
    const previousState = { ...state };
    const next = typeof updater === 'function' ? updater(state) : updater;
    Object.assign(state, next);
    listeners.forEach((listener) => listener(state, previousState));
  };
  useAuthStore.subscribe = (listener) => {
    listeners.add(listener);
    return () => {
      listeners.delete(listener);
    };
  };
  return { useAuthStore };
});
vi.mock('@/stores/offlineStore', () => {
  const state = {
    isOnline: true,
    setOnlineStatus: (value: boolean) => {
      state.isOnline = value;
    },
  };
  const useOfflineStore = ((selector?: (value: typeof state) => unknown) =>
    selector ? selector(state) : state) as unknown as {
    (selector?: (value: typeof state) => unknown): unknown;
    getState: () => typeof state;
    setState: (updater: typeof state | ((current: typeof state) => typeof state)) => void;
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
    ownerNpub: null as string | null,
    hasPendingSync: false,
    lastSyncedAt: null as number | null,
    lastSyncError: null as string | null,
    updatedAt: null as number | null,
    setPublicProfile: (value: boolean) => {
      store.publicProfile = value;
      store.hasPendingSync = true;
    },
    setShowOnlineStatus: (value: boolean) => {
      store.showOnlineStatus = value;
      store.hasPendingSync = true;
    },
    applyLocalChange: (payload: {
      npub?: string;
      publicProfile?: boolean;
      showOnlineStatus?: boolean;
    }) => {
      if (payload.npub) {
        store.ownerNpub = payload.npub;
      }
      if (typeof payload.publicProfile === 'boolean') {
        store.publicProfile = payload.publicProfile;
      }
      if (typeof payload.showOnlineStatus === 'boolean') {
        store.showOnlineStatus = payload.showOnlineStatus;
      }
      store.hasPendingSync = true;
      store.updatedAt = Date.now();
    },
    markSyncSuccess: () => {
      store.hasPendingSync = false;
      store.lastSyncError = null;
      store.lastSyncedAt = Date.now();
    },
    markSyncFailure: (reason?: string | null) => {
      store.hasPendingSync = true;
      store.lastSyncError = reason ?? 'sync_failed';
    },
    hydrateFromUser: (
      user: { npub: string; publicProfile: boolean; showOnlineStatus: boolean } | null,
    ) => {
      if (!user) {
        return;
      }
      if (store.ownerNpub === user.npub && store.hasPendingSync) {
        return;
      }
      store.ownerNpub = user.npub;
      store.publicProfile = user.publicProfile;
      store.showOnlineStatus = user.showOnlineStatus;
      store.hasPendingSync = false;
      store.lastSyncError = null;
    },
    reset: () => {
      store.publicProfile = true;
      store.showOnlineStatus = false;
      store.ownerNpub = null;
      store.hasPendingSync = false;
      store.lastSyncedAt = null;
      store.lastSyncError = null;
      store.updatedAt = null;
    },
  };

  const usePrivacySettingsStore = ((selector?: (value: typeof store) => unknown) =>
    selector ? selector(store) : store) as unknown as {
    (selector?: (value: typeof store) => unknown): unknown;
    getState: () => typeof store;
  };
  usePrivacySettingsStore.getState = () => store;
  return { usePrivacySettingsStore };
});
vi.mock('@/lib/api/tauri', () => ({
  TauriApi: {
    updatePrivacySettings: vi.fn(),
  },
}));
vi.mock('@/lib/api/nostr', () => ({
  updateNostrMetadata: vi.fn(),
}));
vi.mock('@/lib/api/accessControl', () => ({
  accessControlApi: {
    requestJoin: vi.fn().mockResolvedValue({ event_id: '', sent_topics: [] }),
  },
}));
vi.mock('@/lib/api/communityNode', () => ({
  communityNodeApi: {
    getConfig: vi.fn().mockResolvedValue(null),
    getTrustProvider: vi.fn().mockResolvedValue(null),
    listGroupKeys: vi.fn().mockResolvedValue([]),
    getConsentStatus: vi.fn().mockResolvedValue(null),
    setConfig: vi.fn().mockResolvedValue({ nodes: [] }),
    clearConfig: vi.fn().mockResolvedValue(undefined),
    setTrustProvider: vi.fn().mockResolvedValue(null),
    clearTrustProvider: vi.fn().mockResolvedValue(undefined),
    authenticate: vi.fn().mockResolvedValue({ expires_at: 0, pubkey: '' }),
    clearToken: vi.fn().mockResolvedValue(undefined),
    acceptConsents: vi.fn().mockResolvedValue(null),
  },
}));

const renderSettingsPage = () => {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
    },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <SettingsPage />
    </QueryClientProvider>,
  );
};

describe('SettingsPage', () => {
  beforeEach(async () => {
    vi.clearAllMocks();
    await i18n.changeLanguage('ja');
    localStorage.clear();
    usePrivacySettingsStore.getState().reset();
    useOfflineStore.setState((state) => ({ ...state, isOnline: true }));
    useUIStore.setState({
      sidebarOpen: true,
      theme: 'light',
      isLoading: false,
      error: null,
      toggleSidebar: vi.fn(),
      setSidebarOpen: vi.fn(),
      setTheme: vi.fn(),
      setLoading: vi.fn(),
      setError: vi.fn(),
      clearError: vi.fn(),
    });

    const authState = (useAuthStore as unknown as { getState: () => any }).getState();
    authState.currentUser = {
      id: 'user-1',
      pubkey: 'pubkey',
      npub: 'npub',
      name: 'テストユーザー',
      displayName: 'テストユーザー',
      about: '自己紹介',
      picture: '',
      nip05: '',
      publicProfile: true,
      showOnlineStatus: false,
    };
    authState.updateUser = vi.fn();

    vi.mocked(TauriApi.updatePrivacySettings).mockResolvedValue(undefined);
    vi.mocked(updateNostrMetadata).mockResolvedValue('');
  });

  it('プライバシートグルが初期状態を反映する', () => {
    renderSettingsPage();

    const publicSwitch = screen.getByRole('switch', { name: 'プロフィールを公開' });
    const onlineSwitch = screen.getByRole('switch', { name: 'オンライン状態を表示' });
    expect(publicSwitch).not.toBeDisabled();
    expect(onlineSwitch).not.toBeDisabled();

    expect(publicSwitch).toHaveAttribute('data-state', 'checked');
    expect(onlineSwitch).toHaveAttribute('data-state', 'unchecked');
  });

  it('トグル操作で設定が更新される', async () => {
    const user = userEvent.setup();
    renderSettingsPage();

    const publicSwitch = screen.getByRole('switch', { name: 'プロフィールを公開' });
    expect(publicSwitch).not.toBeDisabled();
    const onlineSwitch = screen.getByRole('switch', { name: 'オンライン状態を表示' });

    await user.click(publicSwitch);
    await user.click(onlineSwitch);

    await waitFor(() => expect(TauriApi.updatePrivacySettings).toHaveBeenCalledTimes(2));
    expect(TauriApi.updatePrivacySettings).toHaveBeenNthCalledWith(1, {
      npub: 'npub',
      publicProfile: false,
      showOnlineStatus: false,
    });
    expect(TauriApi.updatePrivacySettings).toHaveBeenNthCalledWith(2, {
      npub: 'npub',
      publicProfile: false,
      showOnlineStatus: true,
    });
    const state = usePrivacySettingsStore.getState();
    expect(state.publicProfile).toBe(false);
    expect(state.showOnlineStatus).toBe(true);
  });

  it('オフライン時はローカル保存のみ行いオンライン復帰時に同期する', async () => {
    useOfflineStore.setState((state) => ({ ...state, isOnline: false }));
    const user = userEvent.setup();
    renderSettingsPage();

    const publicSwitch = screen.getByRole('switch', { name: 'プロフィールを公開' });
    await user.click(publicSwitch);

    expect(TauriApi.updatePrivacySettings).not.toHaveBeenCalled();
    expect(usePrivacySettingsStore.getState().hasPendingSync).toBe(true);

    useOfflineStore.setState((state) => ({ ...state, isOnline: true }));
    act(() => {
      window.dispatchEvent(new Event('online'));
    });

    await waitFor(() => expect(TauriApi.updatePrivacySettings).toHaveBeenCalledTimes(1));
    expect(TauriApi.updatePrivacySettings).toHaveBeenCalledWith({
      npub: 'npub',
      publicProfile: false,
      showOnlineStatus: false,
    });
  });

  it('プロフィール編集ボタンでダイアログが開く', async () => {
    const user = userEvent.setup();
    renderSettingsPage();

    const editButton = screen.getByRole('button', { name: '編集' });
    await user.click(editButton);

    expect(screen.getByTestId('profile-form')).toBeInTheDocument();
    expect(screen.getByLabelText('名前 *')).toHaveValue('テストユーザー');
  });
});
