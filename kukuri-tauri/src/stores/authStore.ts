import { create } from 'zustand'
import { persist, createJSONStorage } from 'zustand/middleware'
import type { AuthState, User } from './types'
import { TauriApi, NostrAPI, RelayInfo } from '@/lib/api/tauri'

interface AuthStore extends AuthState {
  relayStatus: RelayInfo[]
  login: (privateKey: string, user: User) => void
  loginWithNsec: (nsec: string) => Promise<void>
  generateNewKeypair: () => Promise<{ nsec: string }>
  logout: () => void
  updateUser: (user: Partial<User>) => void
  updateRelayStatus: () => Promise<void>
}

export const useAuthStore = create<AuthStore>()(
  persist(
    (set) => ({
      isAuthenticated: false,
      currentUser: null,
      privateKey: null,
      relayStatus: [],

      login: (privateKey: string, user: User) => 
        set({
          isAuthenticated: true,
          currentUser: user,
          privateKey
        }),

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
            nip05: ''
          };
          set({
            isAuthenticated: true,
            currentUser: user,
            privateKey: nsec
          });
          
          // Nostrクライアントを初期化
          await NostrAPI.initialize();
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
            nip05: ''
          };
          set({
            isAuthenticated: true,
            currentUser: user,
            privateKey: response.nsec
          });
          
          // Nostrクライアントを初期化
          await NostrAPI.initialize();
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
          await NostrAPI.disconnect();
          await TauriApi.logout();
        } catch (error) {
          console.error('Logout failed:', error);
        }
        set({
          isAuthenticated: false,
          currentUser: null,
          privateKey: null
        });
      },

      updateUser: (userUpdate: Partial<User>) =>
        set((state) => ({
          currentUser: state.currentUser ? {
            ...state.currentUser,
            ...userUpdate
          } : null
        })),
      
      updateRelayStatus: async () => {
        try {
          const status = await NostrAPI.getRelayStatus();
          set({ relayStatus: status });
        } catch (error) {
          console.error('Failed to get relay status:', error);
        }
      }
    }),
    {
      name: 'auth-storage',
      storage: createJSONStorage(() => localStorage),
      partialize: (state) => ({
        isAuthenticated: state.isAuthenticated,
        currentUser: state.currentUser
      })
    }
  )
)