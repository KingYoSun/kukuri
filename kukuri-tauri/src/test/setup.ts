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

    // 初期状態のディープコピー関数
    const deepCopyState = (obj: any): any => {
      if (obj instanceof Map) {
        return new Map(Array.from(obj.entries()).map(([k, v]) => [k, deepCopyState(v)]));
      }
      if (obj instanceof Set) {
        return new Set(Array.from(obj));
      }
      if (obj instanceof Date) {
        return new Date(obj);
      }
      if (obj === null || typeof obj !== 'object') {
        return obj;
      }
      if (Array.isArray(obj)) {
        return obj.map(deepCopyState);
      }
      const copy: any = {};
      for (const key in obj) {
        if (Object.prototype.hasOwnProperty.call(obj, key)) {
          copy[key] = deepCopyState(obj[key]);
        }
      }
      return copy;
    };

    // 初期状態を保存してリセット可能にする
    const initialState = deepCopyState(state);
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

// P2PEventListenerのモック
vi.mock('@/hooks/useP2PEventListener', () => ({
  useP2PEventListener: vi.fn(),
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

// PointerEventのモック - Radix UIコンポーネントのテスト用
class PointerEvent extends MouseEvent {
  constructor(name: string, init?: PointerEventInit) {
    super(name, init);
  }
}

global.PointerEvent = PointerEvent as any;

// requestAnimationFrameのモック
global.requestAnimationFrame = (cb: any) => {
  setTimeout(cb, 0);
  return 0;
};

global.cancelAnimationFrame = () => {};
