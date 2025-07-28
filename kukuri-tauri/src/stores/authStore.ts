import { create } from 'zustand';
import { persist, createJSONStorage } from 'zustand/middleware';
import type { AuthState, User } from './types';
import { TauriApi } from '@/lib/api/tauri';
import { initializeNostr, disconnectNostr, getRelayStatus, type RelayInfo } from '@/lib/api/nostr';
import { SecureStorageApi, type AccountMetadata } from '@/lib/api/secureStorage';

interface AuthStore extends AuthState {
  relayStatus: RelayInfo[];
  accounts: AccountMetadata[];
  login: (privateKey: string, user: User) => Promise<void>;
  loginWithNsec: (nsec: string, saveToSecureStorage?: boolean) => Promise<void>;
  generateNewKeypair: (saveToSecureStorage?: boolean) => Promise<{ nsec: string }>;
  logout: () => Promise<void>;
  updateUser: (user: Partial<User>) => void;
  updateRelayStatus: () => Promise<void>;
  setRelayStatus: (status: RelayInfo[]) => void;
  initialize: () => Promise<void>;
  switchAccount: (npub: string) => Promise<void>;
  removeAccount: (npub: string) => Promise<void>;
  loadAccounts: () => Promise<void>;
  get isLoggedIn(): boolean;
}

export const useAuthStore = create<AuthStore>()(
  persist(
    (set, get) => ({
      isAuthenticated: false,
      currentUser: null,
      privateKey: null,
      relayStatus: [],
      accounts: [],

      login: async (privateKey: string, user: User) => {
        set({
          isAuthenticated: true,
          currentUser: user,
          privateKey,
        });
        try {
          await initializeNostr();
        } catch (error) {
          console.error('Failed to initialize Nostr:', error);
        }
      },

      loginWithNsec: async (nsec: string, saveToSecureStorage = false) => {
        try {
          const response = await TauriApi.login({ nsec });
          const user: User = {
            id: response.public_key,
            pubkey: response.public_key,
            npub: response.npub,
            name: 'ユーザー',
            displayName: 'ユーザー',
            about: '',
            picture: '',
            nip05: '',
          };
          
          // セキュアストレージに保存
          if (saveToSecureStorage) {
            await SecureStorageApi.addAccount({
              nsec,
              name: user.name,
              display_name: user.displayName,
              picture: user.picture,
            });
          }
          
          set({
            isAuthenticated: true,
            currentUser: user,
            privateKey: nsec,
          });

          // Nostrクライアントを初期化
          await initializeNostr();
          // リレー状態を更新
          await useAuthStore.getState().updateRelayStatus();
          // アカウントリストを更新
          await useAuthStore.getState().loadAccounts();
        } catch (error) {
          console.error('Login failed:', error);
          throw error;
        }
      },

      generateNewKeypair: async (saveToSecureStorage = true) => {
        try {
          const response = await TauriApi.generateKeypair();
          const user: User = {
            id: response.public_key,
            pubkey: response.public_key,
            npub: response.public_key, // TODO: Convert to npub format
            name: '新規ユーザー',
            displayName: '新規ユーザー',
            about: '',
            picture: '',
            nip05: '',
          };
          
          // セキュアストレージに保存
          if (saveToSecureStorage) {
            await SecureStorageApi.addAccount({
              nsec: response.nsec,
              name: user.name,
              display_name: user.displayName,
              picture: user.picture,
            });
          }
          
          set({
            isAuthenticated: true,
            currentUser: user,
            privateKey: response.nsec,
          });

          // Nostrクライアントを初期化
          await initializeNostr();
          // リレー状態を更新
          await useAuthStore.getState().updateRelayStatus();
          // アカウントリストを更新
          await useAuthStore.getState().loadAccounts();

          return { nsec: response.nsec };
        } catch (error) {
          console.error('Keypair generation failed:', error);
          throw error;
        }
      },

      logout: async () => {
        try {
          await disconnectNostr();
        } catch (error) {
          console.error('Failed to disconnect Nostr:', error);
        }
        try {
          await TauriApi.logout();
        } catch (error) {
          console.error('Logout failed:', error);
        }
        set({
          isAuthenticated: false,
          currentUser: null,
          privateKey: null,
          relayStatus: [],
        });
      },

      updateUser: (userUpdate: Partial<User>) =>
        set((state) => ({
          currentUser: state.currentUser
            ? {
                ...state.currentUser,
                ...userUpdate,
              }
            : null,
        })),

      updateRelayStatus: async () => {
        try {
          const status = await getRelayStatus();
          set({ relayStatus: status });
        } catch (error) {
          console.error('Failed to get relay status:', error);
        }
      },

      setRelayStatus: (status: RelayInfo[]) => {
        set({ relayStatus: status });
      },

      initialize: async () => {
        try {
          // セキュアストレージから現在のアカウントを取得
          const currentAccount = await SecureStorageApi.getCurrentAccount();
          
          if (currentAccount) {
            // 自動ログイン
            const user: User = {
              id: currentAccount.pubkey,
              pubkey: currentAccount.pubkey,
              npub: currentAccount.npub,
              name: currentAccount.metadata.name,
              displayName: currentAccount.metadata.display_name,
              about: '',
              picture: currentAccount.metadata.picture || '',
              nip05: '',
            };
            
            set({
              isAuthenticated: true,
              currentUser: user,
              privateKey: currentAccount.nsec,
            });
            
            // Nostrクライアントを初期化
            await initializeNostr();
            // リレー状態を更新
            await useAuthStore.getState().updateRelayStatus();
          } else {
            // アカウントが見つからない場合は初期状態
            set({
              isAuthenticated: false,
              currentUser: null,
              privateKey: null,
              relayStatus: [],
            });
          }
          
          // アカウントリストを読み込み
          await useAuthStore.getState().loadAccounts();
        } catch (error) {
          console.error('Failed to initialize auth store:', error);
          // エラー時は初期状態にリセット
          set({
            isAuthenticated: false,
            currentUser: null,
            privateKey: null,
            relayStatus: [],
            accounts: [],
          });
        }
      },

      switchAccount: async (npub: string) => {
        try {
          // セキュアストレージからログイン
          const response = await SecureStorageApi.secureLogin(npub);
          
          // アカウント情報を取得
          const accounts = await SecureStorageApi.listAccounts();
          const account = accounts.find(a => a.npub === npub);
          
          if (!account) {
            throw new Error('Account not found');
          }
          
          const user: User = {
            id: response.public_key,
            pubkey: response.public_key,
            npub: response.npub,
            name: account.name,
            displayName: account.display_name,
            about: '',
            picture: account.picture || '',
            nip05: '',
          };
          
          set({
            isAuthenticated: true,
            currentUser: user,
            privateKey: null, // セキュアストレージから取得したものは保持しない
          });
          
          // Nostrクライアントを初期化
          await initializeNostr();
          // リレー状態を更新
          await useAuthStore.getState().updateRelayStatus();
        } catch (error) {
          console.error('Failed to switch account:', error);
          throw error;
        }
      },
      
      removeAccount: async (npub: string) => {
        try {
          await SecureStorageApi.removeAccount(npub);
          
          // 現在のアカウントが削除された場合はログアウト
          const currentUser = get().currentUser;
          if (currentUser?.npub === npub) {
            await get().logout();
          }
          
          // アカウントリストを更新
          await get().loadAccounts();
        } catch (error) {
          console.error('Failed to remove account:', error);
          throw error;
        }
      },
      
      loadAccounts: async () => {
        try {
          const accounts = await SecureStorageApi.listAccounts();
          set({ accounts });
        } catch (error) {
          console.error('Failed to load accounts:', error);
          set({ accounts: [] });
        }
      },

      get isLoggedIn() {
        return get().isAuthenticated;
      },
    }),
    {
      name: 'auth-storage',
      storage: createJSONStorage(() => localStorage),
      partialize: (state) => ({
        // privateKeyは保存しない（セキュリティのため）
        isAuthenticated: false, // 常にfalseで保存
        currentUser: state.currentUser,
      }),
    },
  ),
);
