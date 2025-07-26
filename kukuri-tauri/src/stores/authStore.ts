import { create } from 'zustand';
import { persist, createJSONStorage } from 'zustand/middleware';
import type { AuthState, User } from './types';
import { TauriApi } from '@/lib/api/tauri';
import { initializeNostr, disconnectNostr, getRelayStatus, type RelayInfo } from '@/lib/api/nostr';

interface AuthStore extends AuthState {
  relayStatus: RelayInfo[];
  login: (privateKey: string, user: User) => Promise<void>;
  loginWithNsec: (nsec: string) => Promise<void>;
  generateNewKeypair: () => Promise<{ nsec: string }>;
  logout: () => Promise<void>;
  updateUser: (user: Partial<User>) => void;
  updateRelayStatus: () => Promise<void>;
  setRelayStatus: (status: RelayInfo[]) => void;
  get isLoggedIn(): boolean;
}

export const useAuthStore = create<AuthStore>()(
  persist(
    (set, get) => ({
      isAuthenticated: false,
      currentUser: null,
      privateKey: null,
      relayStatus: [],

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

      loginWithNsec: async (nsec: string) => {
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
          set({
            isAuthenticated: true,
            currentUser: user,
            privateKey: nsec,
          });

          // Nostrクライアントを初期化
          await initializeNostr();
          // リレー状態を更新
          await useAuthStore.getState().updateRelayStatus();
        } catch (error) {
          console.error('Login failed:', error);
          throw error;
        }
      },

      generateNewKeypair: async () => {
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
          set({
            isAuthenticated: true,
            currentUser: user,
            privateKey: response.nsec,
          });

          // Nostrクライアントを初期化
          await initializeNostr();
          // リレー状態を更新
          await useAuthStore.getState().updateRelayStatus();

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

      get isLoggedIn() {
        return get().isAuthenticated;
      },
    }),
    {
      name: 'auth-storage',
      storage: createJSONStorage(() => localStorage),
      partialize: (state) => ({
        isAuthenticated: state.isAuthenticated,
        currentUser: state.currentUser,
      }),
    },
  ),
);
