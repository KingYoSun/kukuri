import { describe, it, expect, beforeEach } from 'vitest'
import { useUIStore } from '../uiStore'

describe('uiStore', () => {
  beforeEach(() => {
    useUIStore.setState({
      sidebarOpen: true,
      theme: 'system',
      isLoading: false,
      error: null,
    })
  })

  it('初期状態が正しく設定されていること', () => {
    const state = useUIStore.getState()
    expect(state.sidebarOpen).toBe(true)
    expect(state.theme).toBe('system')
    expect(state.isLoading).toBe(false)
    expect(state.error).toBeNull()
  })

  it('toggleSidebarメソッドが正しく動作すること', () => {
    useUIStore.getState().toggleSidebar()
    expect(useUIStore.getState().sidebarOpen).toBe(false)
    
    useUIStore.getState().toggleSidebar()
    expect(useUIStore.getState().sidebarOpen).toBe(true)
  })

  it('setSidebarOpenメソッドが正しく動作すること', () => {
    useUIStore.getState().setSidebarOpen(false)
    expect(useUIStore.getState().sidebarOpen).toBe(false)
    
    useUIStore.getState().setSidebarOpen(true)
    expect(useUIStore.getState().sidebarOpen).toBe(true)
  })

  it('setThemeメソッドが正しく動作すること', () => {
    useUIStore.getState().setTheme('dark')
    expect(useUIStore.getState().theme).toBe('dark')
    
    useUIStore.getState().setTheme('light')
    expect(useUIStore.getState().theme).toBe('light')
    
    useUIStore.getState().setTheme('system')
    expect(useUIStore.getState().theme).toBe('system')
  })

  it('setLoadingメソッドが正しく動作すること', () => {
    useUIStore.getState().setLoading(true)
    expect(useUIStore.getState().isLoading).toBe(true)
    
    useUIStore.getState().setLoading(false)
    expect(useUIStore.getState().isLoading).toBe(false)
  })

  it('setErrorメソッドが正しく動作すること', () => {
    const errorMessage = 'エラーが発生しました'
    useUIStore.getState().setError(errorMessage)
    expect(useUIStore.getState().error).toBe(errorMessage)
    
    useUIStore.getState().setError(null)
    expect(useUIStore.getState().error).toBeNull()
  })

  it('clearErrorメソッドが正しく動作すること', () => {
    useUIStore.setState({ error: 'テストエラー' })
    
    useUIStore.getState().clearError()
    expect(useUIStore.getState().error).toBeNull()
  })
})