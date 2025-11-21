import React from 'react';
import { vi, describe, it, expect, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { clearFallbackAccounts, useAuthStore } from '@/stores/authStore';
import { SecureStorageApi } from '@/lib/api/secureStorage';
import { TauriApi } from '@/lib/api/tauri';
import * as nostrApi from '@/lib/api/nostr';

vi.mock('@/lib/api/secureStorage');
vi.mock('@/lib/api/nostr');

const mockTauriApi = TauriApi as unknown as {
  generateKeypair: ReturnType<typeof vi.fn>;
  login: ReturnType<typeof vi.fn>;
  logout: ReturnType<typeof vi.fn>;
};

const mockSecureStorageApi = SecureStorageApi as unknown as {
  addAccount: ReturnType<typeof vi.fn>;
  listAccounts: ReturnType<typeof vi.fn>;
  switchAccount: ReturnType<typeof vi.fn>;
  removeAccount: ReturnType<typeof vi.fn>;
  getCurrentAccount: ReturnType<typeof vi.fn>;
  secureLogin: ReturnType<typeof vi.fn>;
};

const mockNostrApi = nostrApi as unknown as {
  initializeNostr: ReturnType<typeof vi.fn>;
  disconnectNostr: ReturnType<typeof vi.fn>;
  getRelayStatus: ReturnType<typeof vi.fn>;
};

function AccountSwitcher() {
  const { isAuthenticated, currentUser, accounts, switchAccount, removeAccount, loadAccounts } =
    useAuthStore();

  React.useEffect(() => {
    loadAccounts();
  }, [loadAccounts]);

  if (!isAuthenticated) {
    return <div>ログインしてください</div>;
  }

  return (
    <div>
      <h2>現在のアカウント</h2>
      <p data-testid="current-account">
        {currentUser?.displayName} ({currentUser?.npub})
      </p>

      <h3>アカウント一覧</h3>
      <ul>
        {accounts.map((account) => (
          <li key={account.npub}>
            <span>{account.display_name}</span>
            {account.npub !== currentUser?.npub && (
              <button
                data-testid={`switch-${account.npub}`}
                onClick={() => switchAccount(account.npub)}
              >
                切り替え
              </button>
            )}
            <button
              data-testid={`remove-${account.npub}`}
              onClick={() => removeAccount(account.npub)}
            >
              削除
            </button>
          </li>
        ))}
      </ul>
    </div>
  );
}

describe('Multiple Accounts Integration', () => {
  const user = userEvent.setup();

  beforeEach(() => {
    clearFallbackAccounts();
    useAuthStore.setState({
      isAuthenticated: false,
      currentUser: null,
      privateKey: null,
      relayStatus: [],
      accounts: [],
    });

    vi.clearAllMocks();
    mockNostrApi.initializeNostr = vi.fn().mockResolvedValue(undefined);
    mockNostrApi.disconnectNostr = vi.fn().mockResolvedValue(undefined);
    mockNostrApi.getRelayStatus = vi.fn().mockResolvedValue([]);
    mockTauriApi.logout = vi.fn().mockResolvedValue(undefined);
  });

  describe('Account Switching Workflow', () => {
    it('should handle complete account switching workflow', async () => {
      const mockAccounts = [
        {
          npub: 'npub1alice',
          pubkey: 'pubkey_alice',
          name: 'Alice',
          display_name: 'Alice Smith',
          picture: 'https://example.com/alice.png',
          last_used: '2024-01-01T00:00:00Z',
        },
        {
          npub: 'npub1bob',
          pubkey: 'pubkey_bob',
          name: 'Bob',
          display_name: 'Bob Johnson',
          picture: 'https://example.com/bob.png',
          last_used: '2024-01-02T00:00:00Z',
        },
      ];

      useAuthStore.setState({
        isAuthenticated: true,
        currentUser: {
          id: 'pubkey_alice',
          pubkey: 'pubkey_alice',
          npub: 'npub1alice',
          name: 'Alice',
          displayName: 'Alice Smith',
          about: '',
          picture: 'https://example.com/alice.png',
          nip05: '',
        },
        privateKey: 'nsec1alice',
        accounts: mockAccounts,
      });

      mockSecureStorageApi.listAccounts = vi.fn().mockResolvedValue(mockAccounts);
      mockSecureStorageApi.secureLogin = vi.fn().mockResolvedValue({
        public_key: 'pubkey_bob',
        npub: 'npub1bob',
      });

      render(<AccountSwitcher />);

      expect(screen.getByTestId('current-account')).toHaveTextContent('Alice Smith (npub1alice)');

      await user.click(screen.getByTestId('switch-npub1bob'));

      await waitFor(() => {
        expect(useAuthStore.getState().currentUser?.npub).toBe('npub1bob');
      });

      expect(mockNostrApi.initializeNostr).toHaveBeenCalled();
    });

    it('should handle account removal workflow', async () => {
      const mockAccounts = [
        {
          npub: 'npub1alice',
          pubkey: 'pubkey_alice',
          name: 'Alice',
          display_name: 'Alice Smith',
          last_used: '2024-01-01T00:00:00Z',
        },
        {
          npub: 'npub1bob',
          pubkey: 'pubkey_bob',
          name: 'Bob',
          display_name: 'Bob Johnson',
          last_used: '2024-01-02T00:00:00Z',
        },
      ];

      useAuthStore.setState({
        isAuthenticated: true,
        currentUser: {
          id: 'pubkey_alice',
          pubkey: 'pubkey_alice',
          npub: 'npub1alice',
          name: 'Alice',
          displayName: 'Alice Smith',
          about: '',
          picture: '',
          nip05: '',
        },
        privateKey: 'nsec1alice',
        accounts: mockAccounts,
      });

      mockSecureStorageApi.listAccounts = vi
        .fn()
        .mockResolvedValueOnce(mockAccounts)
        .mockResolvedValueOnce([mockAccounts[0]]);
      mockSecureStorageApi.removeAccount = vi.fn().mockResolvedValue(undefined);

      render(<AccountSwitcher />);

      await user.click(screen.getByTestId('remove-npub1bob'));

      await waitFor(() => {
        expect(mockSecureStorageApi.removeAccount).toHaveBeenCalledWith('npub1bob');
      });

      await waitFor(() => {
        expect(useAuthStore.getState().accounts).toHaveLength(1);
      });

      expect(useAuthStore.getState().isAuthenticated).toBe(true);
      expect(useAuthStore.getState().currentUser?.npub).toBe('npub1alice');
    });

    it('should logout when removing current account', async () => {
      const mockAccounts = [
        {
          npub: 'npub1alice',
          pubkey: 'pubkey_alice',
          name: 'Alice',
          display_name: 'Alice Smith',
          last_used: '2024-01-01T00:00:00Z',
        },
      ];

      useAuthStore.setState({
        isAuthenticated: true,
        currentUser: {
          id: 'pubkey_alice',
          pubkey: 'pubkey_alice',
          npub: 'npub1alice',
          name: 'Alice',
          displayName: 'Alice Smith',
          about: '',
          picture: '',
          nip05: '',
        },
        privateKey: 'nsec1alice',
        accounts: mockAccounts,
      });

      mockSecureStorageApi.listAccounts = vi
        .fn()
        .mockResolvedValueOnce(mockAccounts)
        .mockResolvedValueOnce([]);
      mockSecureStorageApi.removeAccount = vi.fn().mockResolvedValue(undefined);

      render(<AccountSwitcher />);

      await user.click(screen.getByTestId('remove-npub1alice'));

      await waitFor(() => {
        expect(mockSecureStorageApi.removeAccount).toHaveBeenCalledWith('npub1alice');
      });

      await waitFor(() => {
        expect(useAuthStore.getState().isAuthenticated).toBe(false);
      });

      expect(mockTauriApi.logout).toHaveBeenCalled();
      expect(mockNostrApi.disconnectNostr).toHaveBeenCalled();
    });
  });

  describe('New Account Creation with Secure Storage', () => {
    it('should create new account and save to secure storage', async () => {
      const mockKeypairResponse = {
        public_key: 'pubkey_new',
        npub: 'npub1new',
        nsec: 'nsec1new',
      };

      mockTauriApi.generateKeypair = vi.fn().mockResolvedValue(mockKeypairResponse);
      mockSecureStorageApi.addAccount = vi.fn().mockResolvedValue({
        npub: 'npub1new',
        pubkey: 'pubkey_new',
      });
      mockSecureStorageApi.listAccounts = vi.fn().mockResolvedValue([
        {
          npub: 'npub1new',
          pubkey: 'pubkey_new',
          name: '新規ユーザー',
          display_name: '新規ユーザー',
          last_used: new Date().toISOString(),
        },
      ]);

      const result = await useAuthStore.getState().generateNewKeypair(true);

      expect(result.nsec).toBe('nsec1new');
      expect(mockSecureStorageApi.addAccount).toHaveBeenCalledWith({
        nsec: 'nsec1new',
        name: '新規ユーザー',
        display_name: '新規ユーザー',
        picture: '',
      });

      const state = useAuthStore.getState();
      expect(state.isAuthenticated).toBe(true);
      expect(state.accounts).toHaveLength(1);
    });
  });
});
