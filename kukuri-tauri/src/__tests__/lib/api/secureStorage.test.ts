import { vi, describe, it, expect, beforeEach } from 'vitest';
import {
  SecureStorageApi,
  type AccountMetadata,
  type AddAccountRequest,
} from '@/lib/api/secureStorage';

// @tauri-apps/api/coreのモック
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

import { invoke } from '@tauri-apps/api/core';
const mockInvoke = invoke as unknown as ReturnType<typeof vi.fn>;

describe('SecureStorageApi', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('addAccount', () => {
    it('should add a new account successfully', async () => {
      const request: AddAccountRequest = {
        nsec: 'nsec1test123',
        name: 'Test User',
        display_name: 'Test Display',
        picture: 'https://example.com/avatar.png',
      };

      const mockResponse = {
        npub: 'npub1test123',
        pubkey: 'pubkey123',
      };

      mockInvoke.mockResolvedValueOnce(mockResponse);

      const result = await SecureStorageApi.addAccount(request);

      expect(mockInvoke).toHaveBeenCalledWith('add_account', { request });
      expect(result).toEqual(mockResponse);
    });

    it('should handle errors when adding account', async () => {
      const request: AddAccountRequest = {
        nsec: 'invalid_nsec',
        name: 'Test User',
        display_name: 'Test Display',
      };

      mockInvoke.mockRejectedValueOnce(new Error('Invalid nsec format'));

      await expect(SecureStorageApi.addAccount(request)).rejects.toThrow('Invalid nsec format');
    });
  });

  describe('listAccounts', () => {
    it('should return list of accounts', async () => {
      const mockAccounts: AccountMetadata[] = [
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
          last_used: '2024-01-02T00:00:00Z',
        },
      ];

      mockInvoke.mockResolvedValueOnce(mockAccounts);

      const result = await SecureStorageApi.listAccounts();

      expect(mockInvoke).toHaveBeenCalledWith('list_accounts');
      expect(result).toEqual(mockAccounts);
      expect(result).toHaveLength(2);
    });

    it('should return empty array when no accounts', async () => {
      mockInvoke.mockResolvedValueOnce([]);

      const result = await SecureStorageApi.listAccounts();

      expect(result).toEqual([]);
    });
  });

  describe('switchAccount', () => {
    it('should switch account successfully', async () => {
      const npub = 'npub1test123';
      const mockResponse = {
        npub: 'npub1test123',
        pubkey: 'pubkey123',
      };

      mockInvoke.mockResolvedValueOnce(mockResponse);

      const result = await SecureStorageApi.switchAccount(npub);

      expect(mockInvoke).toHaveBeenCalledWith('switch_account', { npub });
      expect(result).toEqual(mockResponse);
    });

    it('should handle errors when switching to non-existent account', async () => {
      const npub = 'npub_not_exist';

      mockInvoke.mockRejectedValueOnce(new Error('Account not found'));

      await expect(SecureStorageApi.switchAccount(npub)).rejects.toThrow('Account not found');
    });
  });

  describe('removeAccount', () => {
    it('should remove account successfully', async () => {
      const npub = 'npub1test123';

      mockInvoke.mockResolvedValueOnce(undefined);

      await SecureStorageApi.removeAccount(npub);

      expect(mockInvoke).toHaveBeenCalledWith('remove_account', { npub });
    });

    it('should handle errors when removing account', async () => {
      const npub = 'npub1test123';

      mockInvoke.mockRejectedValueOnce(new Error('Failed to remove account'));

      await expect(SecureStorageApi.removeAccount(npub)).rejects.toThrow(
        'Failed to remove account',
      );
    });
  });

  describe('getCurrentAccount', () => {
    it('should return current account when logged in', async () => {
      const mockAccount = {
        npub: 'npub1current',
        nsec: 'nsec1current',
        pubkey: 'pubkey_current',
        metadata: {
          npub: 'npub1current',
          pubkey: 'pubkey_current',
          name: 'Current User',
          display_name: 'Current User Display',
          picture: 'https://example.com/current.png',
          last_used: '2024-01-01T00:00:00Z',
        },
      };

      mockInvoke.mockResolvedValueOnce(mockAccount);

      const result = await SecureStorageApi.getCurrentAccount();

      expect(mockInvoke).toHaveBeenCalledWith('get_current_account');
      expect(result).toEqual(mockAccount);
    });

    it('should return null when no current account', async () => {
      mockInvoke.mockResolvedValueOnce(null);

      const result = await SecureStorageApi.getCurrentAccount();

      expect(result).toBeNull();
    });
  });

  describe('secureLogin', () => {
    it('should login with secure storage successfully', async () => {
      const npub = 'npub1test123';
      const mockResponse = {
        public_key: 'pubkey123',
        npub: 'npub1test123',
      };

      mockInvoke.mockResolvedValueOnce(mockResponse);

      const result = await SecureStorageApi.secureLogin(npub);

      expect(mockInvoke).toHaveBeenCalledWith('secure_login', { npub });
      expect(result).toEqual(mockResponse);
    });

    it('should handle errors when private key not found', async () => {
      const npub = 'npub_not_exist';

      mockInvoke.mockRejectedValueOnce(new Error('Private key not found'));

      await expect(SecureStorageApi.secureLogin(npub)).rejects.toThrow('Private key not found');
    });
  });
});
