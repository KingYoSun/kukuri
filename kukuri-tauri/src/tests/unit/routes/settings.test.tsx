import { beforeEach, describe, expect, it, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { SettingsPage } from '@/routes/settings';
import { useUIStore } from '@/stores/uiStore';
import { useAuthStore } from '@/stores/authStore';
import { TauriApi } from '@/lib/api/tauri';
import { updateNostrMetadata } from '@/lib/api/nostr';

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

vi.mock('@/stores/authStore');
vi.mock('@/lib/api/tauri', () => ({
  TauriApi: {
    updatePrivacySettings: vi.fn(),
  },
}));
vi.mock('@/lib/api/nostr', () => ({
  updateNostrMetadata: vi.fn(),
}));
vi.mock('@/lib/api/communityNode', () => ({
  communityNodeApi: {
    getConfig: vi.fn().mockResolvedValue(null),
    listGroupKeys: vi.fn().mockResolvedValue([]),
    getConsentStatus: vi.fn().mockResolvedValue(null),
    setConfig: vi.fn().mockResolvedValue({ nodes: [] }),
    clearConfig: vi.fn().mockResolvedValue(undefined),
    authenticate: vi.fn().mockResolvedValue({ expires_at: 0, pubkey: '' }),
    clearToken: vi.fn().mockResolvedValue(undefined),
    syncKeyEnvelopes: vi.fn().mockResolvedValue({ stored: [] }),
    redeemInvite: vi.fn().mockResolvedValue({ topic_id: '', scope: 'invite', epoch: 1 }),
    acceptConsents: vi.fn().mockResolvedValue(null),
  },
}));

const mockPrivacyStore = {
  publicProfile: true,
  showOnlineStatus: false,
  setPublicProfile: vi.fn(),
  setShowOnlineStatus: vi.fn(),
  hydrateFromUser: vi.fn(),
};

vi.mock('@/stores/privacySettingsStore', () => ({
  usePrivacySettingsStore: vi.fn(() => mockPrivacyStore),
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
  beforeEach(() => {
    localStorage.clear();
    mockPrivacyStore.publicProfile = true;
    mockPrivacyStore.showOnlineStatus = false;
    mockPrivacyStore.setPublicProfile.mockImplementation((value: boolean) => {
      mockPrivacyStore.publicProfile = value;
    });
    mockPrivacyStore.setShowOnlineStatus.mockImplementation((value: boolean) => {
      mockPrivacyStore.showOnlineStatus = value;
    });
    mockPrivacyStore.hydrateFromUser.mockImplementation(() => {});
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

    vi.mocked(useAuthStore).mockReturnValue({
      currentUser: {
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
      },
      updateUser: vi.fn(),
    });

    vi.mocked(TauriApi.updatePrivacySettings).mockResolvedValue(undefined);
    vi.mocked(updateNostrMetadata).mockResolvedValue('');
  });

  it('プライバシートグルが初期状態を反映する', () => {
    renderSettingsPage();

    const publicSwitch = screen.getByRole('switch', { name: 'プロフィールを公開' });
    const onlineSwitch = screen.getByRole('switch', { name: 'オンライン状態を表示' });

    expect(publicSwitch).toHaveAttribute('data-state', 'checked');
    expect(onlineSwitch).toHaveAttribute('data-state', 'unchecked');
  });

  it('トグル操作で設定が更新される', async () => {
    const user = userEvent.setup();
    renderSettingsPage();

    const publicSwitch = screen.getByRole('switch', { name: 'プロフィールを公開' });
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
    expect(mockPrivacyStore.publicProfile).toBe(false);
    expect(mockPrivacyStore.showOnlineStatus).toBe(true);
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
