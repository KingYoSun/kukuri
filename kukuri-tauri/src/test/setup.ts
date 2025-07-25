import '@testing-library/jest-dom'
import { vi, afterEach } from 'vitest'
import { act } from '@testing-library/react'

// リセット関数のセット
const storeResetFns = new Set<() => void>()

// zustandをモック
vi.mock('zustand', async () => {
  const zustand = await vi.importActual('zustand') as typeof import('zustand')
  const { create: actualCreate, createStore: actualCreateStore } = zustand

  // カスタムcreate関数
  const create: typeof actualCreate = (stateCreator) => {
    const store = actualCreate(stateCreator)
    const initialState = store.getState()
    storeResetFns.add(() => {
      store.setState(initialState, true)
    })
    return store
  }

  // カスタムcreateStore関数
  const createStore: typeof actualCreateStore = (stateCreator) => {
    const store = actualCreateStore(stateCreator)
    const initialState = store.getInitialState()
    storeResetFns.add(() => {
      store.setState(initialState, true)
    })
    return store
  }

  return {
    ...await vi.importActual('zustand'),
    create,
    createStore,
  }
})

// 各テスト後にストアをリセット
afterEach(() => {
  act(() => {
    // localStorageをクリア
    if (typeof window !== 'undefined' && window.localStorage) {
      window.localStorage.clear()
    }
    
    // 全てのストアをリセット
    storeResetFns.forEach((resetFn) => {
      resetFn()
    })
  })
})

// CSSファイルのモック
vi.mock('*.css', () => ({}))

// Tauri APIのモック
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

// Window matchMediaのモック
Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: vi.fn().mockImplementation(query => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  })),
})

// ResizeObserverのモック
global.ResizeObserver = vi.fn().mockImplementation(() => ({
  observe: vi.fn(),
  unobserve: vi.fn(),
  disconnect: vi.fn(),
}))