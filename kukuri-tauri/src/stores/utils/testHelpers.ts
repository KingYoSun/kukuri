import { vi } from 'vitest';
import type { StateCreator } from 'zustand';

/**
 * Zustandストアのモックを作成するヘルパー関数
 *
 * @example
 * const mockStore = createStoreMock<AuthStore>({
 *   isAuthenticated: false,
 *   currentUser: null,
 *   login: vi.fn(),
 *   logout: vi.fn(),
 * });
 *
 * vi.mocked(useAuthStore).mockReturnValue(mockStore);
 */
export const createStoreMock = <T extends Record<string, unknown>>(initialState: Partial<T>): T => {
  return initialState as T;
};

/**
 * Zustandストアの初期状態を設定するヘルパー関数
 * テストセットアップで使用
 *
 * @example
 * beforeEach(() => {
 *   setupStoreState(useAuthStore, {
 *     isAuthenticated: false,
 *     currentUser: null,
 *   });
 * });
 */
export const setupStoreState = <T>(
  useStore: StateCreator<T> | { setState: (state: Partial<T>) => void },
  state: Partial<T>,
): void => {
  if ('setState' in useStore) {
    useStore.setState(state);
  }
};

/**
 * 複数のストアをまとめてモックするヘルパー関数
 *
 * @example
 * const mocks = setupStoreMocks({
 *   authStore: {
 *     isAuthenticated: true,
 *     currentUser: mockUser,
 *     login: vi.fn(),
 *   },
 *   topicStore: {
 *     topics: new Map(),
 *     joinedTopics: [],
 *     joinTopic: vi.fn(),
 *   },
 * });
 */
export const setupStoreMocks = <T extends Record<string, Record<string, unknown>>>(mocks: T): T => {
  return Object.entries(mocks).reduce((acc, [key, value]) => {
    acc[key as keyof T] = createStoreMock(value) as T[keyof T];
    return acc;
  }, {} as T);
};

/**
 * Mapオブジェクトを含むストアのモックを作成
 *
 * @example
 * const mockStore = createStoreWithMapMock<PostStore>({
 *   posts: [['post1', mockPost1], ['post2', mockPost2]],
 *   postsByTopic: [],
 *   createPost: vi.fn(),
 * });
 */
export const createStoreWithMapMock = <T extends Record<string, unknown>>(initialState: {
  [K in keyof T]: T[K] extends Map<infer Key, infer Value> ? Array<[Key, Value]> : T[K];
}): T => {
  const store = {} as T;

  Object.entries(initialState).forEach(([key, value]) => {
    if (Array.isArray(value) && value.length > 0 && Array.isArray(value[0])) {
      // Array of tuples -> convert to Map
      (store as Record<string, unknown>)[key] = new Map(value);
    } else {
      (store as Record<string, unknown>)[key] = value;
    }
  });

  return store;
};

/**
 * ストアのアクションをスパイするヘルパー関数
 *
 * @example
 * const spies = spyStoreActions(useAuthStore, ['login', 'logout', 'updateUser']);
 *
 * // テスト実行後
 * expect(spies.login).toHaveBeenCalledWith(privateKey, user);
 */
export const spyStoreActions = <T extends Record<string, unknown>>(
  useStore: { getState: () => T },
  actions: Array<keyof T>,
): Record<keyof T, ReturnType<typeof vi.fn>> => {
  const state = useStore.getState();
  const spies = {} as Record<keyof T, ReturnType<typeof vi.fn>>;

  actions.forEach((action) => {
    if (typeof state[action] === 'function') {
      const spy = vi.fn((state[action] as (...args: unknown[]) => unknown).bind(state));
      (state as Record<string, unknown>)[action as string] = spy;
      spies[action] = spy;
    }
  });

  return spies;
};

/**
 * ストアの状態変更を監視するヘルパー関数
 *
 * @example
 * const unsubscribe = watchStoreChanges(useAuthStore, (state) => {
 *   console.log('State changed:', state);
 * });
 *
 * // クリーンアップ
 * unsubscribe();
 */
export const watchStoreChanges = <T>(
  useStore: { subscribe: (listener: (state: T) => void) => () => void },
  callback: (state: T) => void,
): (() => void) => {
  return useStore.subscribe(callback);
};

/**
 * 非同期ストアアクションをテストするヘルパー関数
 *
 * @example
 * await testAsyncStoreAction(
 *   () => useAuthStore.getState().login(privateKey, user),
 *   () => {
 *     const state = useAuthStore.getState();
 *     expect(state.isAuthenticated).toBe(true);
 *     expect(state.currentUser).toEqual(user);
 *   }
 * );
 */
export const testAsyncStoreAction = async <T>(
  action: () => Promise<T> | T,
  assertion: (result?: T) => void,
): Promise<void> => {
  const result = await action();
  assertion(result);
};

/**
 * ストアのリセットヘルパー
 * テストのafterEachで使用
 *
 * @example
 * afterEach(() => {
 *   resetStore(useAuthStore, initialAuthState);
 *   resetStore(useTopicStore, initialTopicState);
 * });
 */
export const resetStore = <T>(
  useStore: { setState: (state: T) => void; getState: () => T },
  initialState: T,
): void => {
  useStore.setState(initialState);
};

/**
 * 複数のストアを一括リセット
 *
 * @example
 * afterEach(() => {
 *   resetStores({
 *     auth: [useAuthStore, initialAuthState],
 *     topic: [useTopicStore, initialTopicState],
 *   });
 * });
 */
export const resetStores = (
  stores: Record<string, [{ setState: (state: unknown) => void }, unknown]>,
): void => {
  Object.values(stores).forEach(([store, initialState]) => {
    store.setState(initialState);
  });
};

/**
 * persistを使用するストア向けに localStorage をモック化
 */
export const setupPersistMock = () => {
  const localStorageMock = {
    getItem: vi.fn(),
    setItem: vi.fn(),
    removeItem: vi.fn(),
    clear: vi.fn(),
  };

  Object.defineProperty(window, 'localStorage', {
    configurable: true,
    writable: true,
    value: localStorageMock,
  });

  return localStorageMock;
};
