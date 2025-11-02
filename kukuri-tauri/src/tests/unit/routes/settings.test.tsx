import { beforeEach, describe, expect, it, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { SettingsPage } from '@/routes/settings';
import { useUIStore } from '@/stores/uiStore';
import { usePrivacySettingsStore } from '@/stores/privacySettingsStore';
import { useAuthStore } from '@/stores/authStore';

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

const renderSettingsPage = () => {
  return render(<SettingsPage />);
};

describe('SettingsPage', () => {
  beforeEach(() => {
    usePrivacySettingsStore.getState().reset();
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
      },
      updateUser: vi.fn(),
    });
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

    const state = usePrivacySettingsStore.getState();
    expect(state.publicProfile).toBe(false);
    expect(state.showOnlineStatus).toBe(true);
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
