/**
 * WP-S5: shell フック characterization テスト用の renderHook 共通ハーネス。
 *
 * shell の大型フック(viewModels / data / actions / routing)を renderHook で
 * 単体マウントするための store 生成 + Provider wrapper を提供する。
 * WP-H6(store スライス化・selector 化)の安全網となるテスト群が共用する。
 *
 * 契約(T4-T7 が依存するため変更しないこと):
 * - `setWindowHash(hash)` / `resetWindowHash()`: window.history.replaceState の薄いラッパ。
 * - `createShellHookHarness({ hash, router })`:
 *   hash 指定時は store 生成【前】に setWindowHash する。
 *   `createInitialShellState` が window.location.hash を直読するため、この順序が契約。
 *   `router: true` で Provider の内側に HashRouter を重ねる(useDesktopShellRouting 用。
 *   MemoryRouter は不可 — routes.ts が window.location.hash を直読するため URL と乖離する)。
 * - `actPatchState(store, partial)`: レンダー済みフックの外から store を書き換える時の
 *   act ラッパ(React 19 では store 外部操作も act で包む必要がある)。
 */
import { act } from '@testing-library/react';
import type { ComponentType, ReactNode } from 'react';
import { HashRouter } from 'react-router-dom';

import {
  createDesktopShellStore,
  DesktopShellStoreContext,
  type DesktopShellState,
  type DesktopShellStateValue,
} from '@/shell/store';

export type ShellHookHarnessOptions = {
  /** '#/timeline?...' または '/timeline?...' 形式。store 生成前に window.location.hash に反映される。 */
  hash?: string;
  /** true で Provider の内側に HashRouter を重ねる(useDesktopShellRouting 用)。 */
  router?: boolean;
};

export type ShellHookHarness = {
  store: ReturnType<typeof createDesktopShellStore>;
  wrapper: ComponentType<{ children: ReactNode }>;
};

/**
 * window.location.hash を設定する。先頭の '#' は省略可
 * (`setWindowHash('#/timeline?topic=x')` と `setWindowHash('/timeline?topic=x')` は等価)。
 */
export function setWindowHash(hash: string): void {
  const normalized = hash.startsWith('#') ? hash : `#${hash}`;
  window.history.replaceState(null, '', `/${normalized}`);
}

/** URL を '/'(hash なし)へ戻す。各テストの beforeEach で呼ぶこと。 */
export function resetWindowHash(): void {
  window.history.replaceState(null, '', '/');
}

/**
 * shell フック用の store + wrapper を作る。
 * hash 指定時は store 生成前に setWindowHash してから createDesktopShellStore を呼ぶ
 * (createInitialShellState が window.location.hash から初期 state を組み立てるため)。
 */
export function createShellHookHarness(options?: ShellHookHarnessOptions): ShellHookHarness {
  if (options?.hash !== undefined) {
    setWindowHash(options.hash);
  }
  const store = createDesktopShellStore();
  const withRouter = options?.router === true;
  const wrapper = ({ children }: { children: ReactNode }) => (
    <DesktopShellStoreContext.Provider value={store}>
      {withRouter ? <HashRouter>{children}</HashRouter> : children}
    </DesktopShellStoreContext.Provider>
  );
  return { store, wrapper };
}

/** レンダー済みフックの外から store.patchState を呼ぶための act ラッパ。 */
export function actPatchState(
  store: ShellHookHarness['store'],
  partial: Partial<DesktopShellState>
): void {
  act(() => {
    store.getState().patchState(partial);
  });
}

/** 追加ヘルパ: レンダー済みフックの外から store.setField を呼ぶための act ラッパ。 */
export function actSetField<K extends keyof DesktopShellState>(
  store: ShellHookHarness['store'],
  key: K,
  value: DesktopShellStateValue<K>
): void {
  act(() => {
    store.getState().setField(key, value);
  });
}
