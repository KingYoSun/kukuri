import { vi } from 'vitest';

export interface ZustandStoreMock<TState> {
  hook: ReturnType<typeof vi.fn>;
  getState: () => TState;
  setState: (next: TState | ((prev: TState) => TState)) => void;
  apply: (overrides?: Partial<TState>) => void;
  patchState: (partial: Partial<TState>) => void;
  reset: () => void;
}

export const createZustandStoreMock = <TState>(
  factory: () => TState,
): ZustandStoreMock<TState> => {
  let state = factory();

  const hook = vi.fn((selector?: (state: TState) => unknown) =>
    selector ? selector(state) : state,
  );

  const setState = (next: TState | ((prev: TState) => TState)) => {
    state = typeof next === 'function' ? (next as (prev: TState) => TState)(state) : next;
  };

  const apply = (overrides?: Partial<TState>) => {
    const base = factory();
    state = overrides ? { ...base, ...overrides } : base;
  };

  const patchState = (partial: Partial<TState>) => {
    state = { ...state, ...partial };
  };

  const reset = () => {
    state = factory();
    hook.mockClear();
  };

  const getState = () => state;

  return {
    hook,
    getState,
    setState,
    apply,
    patchState,
    reset,
  };
};
