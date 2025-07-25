import { create } from 'zustand'
import { persist, createJSONStorage } from 'zustand/middleware'
import type { AuthState, User } from './types'

interface AuthStore extends AuthState {
  login: (privateKey: string, user: User) => void
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

      logout: () => 
        set({
          isAuthenticated: false,
          currentUser: null,
          privateKey: null
        }),

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