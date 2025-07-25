import { describe, it, expect, beforeEach } from 'vitest'
import { renderHook, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { useLogin, useGenerateKeyPair, useLogout } from '../useAuth'
import { useAuthStore } from '@/stores'
import { ReactNode } from 'react'

const createWrapper = () => {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  })
  
  return ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={queryClient}>
      {children}
    </QueryClientProvider>
  )
}

describe('useAuth hooks', () => {
  beforeEach(() => {
    useAuthStore.setState({
      isAuthenticated: false,
      currentUser: null,
      privateKey: null,
    })
  })

  describe('useLogin', () => {
    it('ログイン成功時にauthStoreが更新されること', async () => {
      const { result } = renderHook(() => useLogin(), {
        wrapper: createWrapper(),
      })

      await result.current.mutateAsync('test-private-key')

      await waitFor(() => {
        const state = useAuthStore.getState()
        expect(state.isAuthenticated).toBe(true)
        expect(state.currentUser).not.toBeNull()
        expect(state.privateKey).toBe('test-private-key')
      })
    })
  })

  describe('useGenerateKeyPair', () => {
    it('鍵ペア生成成功時にauthStoreが更新されること', async () => {
      const { result } = renderHook(() => useGenerateKeyPair(), {
        wrapper: createWrapper(),
      })

      await result.current.mutateAsync()

      await waitFor(() => {
        const state = useAuthStore.getState()
        expect(state.isAuthenticated).toBe(true)
        expect(state.currentUser).not.toBeNull()
        expect(state.privateKey).toMatch(/^nsec/)
      })
    })
  })

  describe('useLogout', () => {
    it('ログアウト時にauthStoreがクリアされること', () => {
      useAuthStore.setState({
        isAuthenticated: true,
        currentUser: {
          pubkey: 'npub123',
          name: 'テストユーザー',
          created_at: Date.now(),
        },
        privateKey: 'nsec123',
      })

      const { result } = renderHook(() => useLogout(), {
        wrapper: createWrapper(),
      })

      result.current()

      const state = useAuthStore.getState()
      expect(state.isAuthenticated).toBe(false)
      expect(state.currentUser).toBeNull()
      expect(state.privateKey).toBeNull()
    })
  })
})