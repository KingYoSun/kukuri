import { create } from 'zustand'
import { persist, createJSONStorage } from 'zustand/middleware'
import type { AuthState, User } from './types'
import { TauriApi } from '@/lib/api/tauri'

interface AuthStore extends AuthState {
  login: (privateKey: string, user: User) => void
  loginWithNsec: (nsec: string) => Promise<void>
  generateNewKeypair: () => Promise<{ nsec: string }>
  logout: () => void
  updateUser: (user: Partial<User>) => void
}

export const useAuthStore = create<AuthStore>()(
  persist(
    (set) => ({
      isAuthenticated: false,
      currentUser: null,
      privateKey: null,

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
          return { nsec: response.nsec };
        } catch (error) {
          console.error('Keypair generation failed:', error);
          throw error;
        }
      },

      logout: async () => {
        try {
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
        }))
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