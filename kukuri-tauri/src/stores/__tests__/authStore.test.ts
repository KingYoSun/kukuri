import { describe, it, expect, beforeEach, vi } from 'vitest'
import { useAuthStore } from '../authStore'
import type { User } from '../types'

// TauriApiをモック
vi.mock('@/lib/api/tauri', () => ({
  TauriApi: {
    logout: vi.fn().mockResolvedValue(undefined),
    login: vi.fn(),
    generateKeypair: vi.fn()
  }
}))

describe('authStore', () => {
  beforeEach(() => {
    useAuthStore.setState({
      isAuthenticated: false,
      currentUser: null,
      privateKey: null,
    })
  })

  it('初期状態が正しく設定されていること', () => {
    const state = useAuthStore.getState()
    expect(state.isAuthenticated).toBe(false)
    expect(state.currentUser).toBeNull()
    expect(state.privateKey).toBeNull()
  })

  it('loginメソッドが正しく動作すること', () => {
    const testUser: User = {
      id: 'test123',
      pubkey: 'pubkey123',
      npub: 'npub123',
      name: 'テストユーザー',
      displayName: 'テストユーザー',
      picture: '',
      about: '',
      nip05: ''
    }
    const testPrivateKey = 'nsec123'

    useAuthStore.getState().login(testPrivateKey, testUser)
    
    const state = useAuthStore.getState()
    expect(state.isAuthenticated).toBe(true)
    expect(state.currentUser).toEqual(testUser)
    expect(state.privateKey).toBe(testPrivateKey)
  })

  it('logoutメソッドが正しく動作すること', async () => {
    const testUser: User = {
      id: 'test123',
      pubkey: 'pubkey123',
      npub: 'npub123',
      name: 'テストユーザー',
      displayName: 'テストユーザー',
      picture: '',
      about: '',
      nip05: ''
    }
    useAuthStore.setState({
      isAuthenticated: true,
      currentUser: testUser,
      privateKey: 'nsec123',
    })

    await useAuthStore.getState().logout()
    
    const state = useAuthStore.getState()
    expect(state.isAuthenticated).toBe(false)
    expect(state.currentUser).toBeNull()
    expect(state.privateKey).toBeNull()
  })

  it('updateUserメソッドが正しく動作すること', () => {
    const testUser: User = {
      id: 'test123',
      pubkey: 'pubkey123',
      npub: 'npub123',
      name: 'テストユーザー',
      displayName: 'テストユーザー',
      picture: '',
      about: '',
      nip05: ''
    }
    useAuthStore.setState({
      isAuthenticated: true,
      currentUser: testUser,
      privateKey: 'nsec123',
    })

    const updates = {
      name: '更新されたユーザー',
      about: '新しい自己紹介',
    }
    useAuthStore.getState().updateUser(updates)
    
    const state = useAuthStore.getState()
    expect(state.currentUser?.name).toBe('更新されたユーザー')
    expect(state.currentUser?.about).toBe('新しい自己紹介')
    expect(state.currentUser?.pubkey).toBe(testUser.pubkey)
  })

  it('currentUserがnullの場合updateUserが何もしないこと', () => {
    useAuthStore.getState().updateUser({ name: '新しい名前' })
    
    const state = useAuthStore.getState()
    expect(state.currentUser).toBeNull()
  })
})