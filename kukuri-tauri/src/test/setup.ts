/* eslint-disable @typescript-eslint/no-explicit-any */
import '@testing-library/jest-dom';
import { vi, afterEach } from 'vitest';
import { act } from '@testing-library/react';

// リセット関数のセット
const storeResetFns = new Set<() => void>();

// zustand/middlewareをモック
vi.mock('zustand/middleware', () => ({
  persist: vi.fn((config) => config),
  createJSONStorage: vi.fn(() => ({
    getItem: vi.fn(),
    setItem: vi.fn(),
    removeItem: vi.fn(),
  })),
}));

// zustandをモック - v5対応
vi.mock('zustand', async () => {
  const { create: _actualCreate } = await vi.importActual<typeof import('zustand')>('zustand');

  const createMockStore = (createState: any) => {
    // 初期状態を作成
    let state: any;
    const setState = (partial: any, replace?: any) => {
      const nextState = typeof partial === 'function' ? partial(state) : partial;
      if (replace ?? typeof partial !== 'object') {
        state = nextState;
      } else {
        state = Object.assign({}, state, nextState);
      }
    };
    const getState = () => state;
    const subscribe = () => () => {};
    const destroy = () => {};

    const api = { setState, getState, subscribe, destroy };
    state = createState(setState, getState, api);

    // フック関数を作成
    const useStore = Object.assign((selector = (state: any) => state) => selector(state), api);

    // 初期状態を保存してリセット可能にする
    const initialState = { ...state };
    storeResetFns.add(() => {
      setState(initialState, true);
    });

    return useStore;
  };

  // カリー化されたcreate関数をサポート
  const create = ((createState?: any) => {
    if (!createState) {
      return (createState: any) => createMockStore(createState);
    }
    return createMockStore(createState);
  }) as typeof _actualCreate;

  return { create };
});

// 各テスト後にストアをリセット
afterEach(() => {
  act(() => {
    // localStorageをクリア
    if (typeof window !== 'undefined' && window.localStorage) {
      window.localStorage.clear();
    }

    // 全てのストアをリセット
    storeResetFns.forEach((resetFn) => {
      resetFn();
    });
  });
});

// CSSファイルのモック
vi.mock('*.css', () => ({}));

// Tauri APIのモック
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

// Window matchMediaのモック
Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: vi.fn().mockImplementation((query) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  })),
});

// ResizeObserverのモック
global.ResizeObserver = vi.fn().mockImplementation(() => ({
  observe: vi.fn(),
  unobserve: vi.fn(),
  disconnect: vi.fn(),
}));
