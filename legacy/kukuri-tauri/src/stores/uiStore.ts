import { create } from 'zustand';

import { createUiPersistConfig, persistKeys } from './config/persist';
import { withPersist } from './utils/persistHelpers';

export type SidebarCategory = 'topics' | 'search' | 'trending' | 'following';
export type UITheme = 'light' | 'dark' | 'system';
export type TimelineUpdateMode = 'standard' | 'realtime';

interface UIState {
  sidebarOpen: boolean;
  theme: UITheme;
  timelineUpdateMode: TimelineUpdateMode;
  isLoading: boolean;
  error: string | null;
  activeSidebarCategory: SidebarCategory | null;
}

interface UIStore extends UIState {
  toggleSidebar: () => void;
  setSidebarOpen: (open: boolean) => void;
  setTheme: (theme: UITheme) => void;
  setTimelineUpdateMode: (mode: TimelineUpdateMode) => void;
  setLoading: (isLoading: boolean) => void;
  setError: (error: string | null) => void;
  clearError: () => void;
  setActiveSidebarCategory: (category: SidebarCategory | null) => void;
  resetActiveSidebarCategory: () => void;
}

type StorageLike = Pick<Storage, 'getItem'>;

const LEGACY_THEME_STORAGE_KEYS = ['kukuri-theme', 'theme'] as const;
const DEFAULT_THEME: UITheme = 'system';

const normalizeThemeCandidate = (candidate: unknown): UITheme | null => {
  if (candidate === 'dark' || candidate === 'light' || candidate === 'system') {
    return candidate;
  }

  if (typeof candidate === 'boolean') {
    return candidate ? 'dark' : 'light';
  }

  if (typeof candidate === 'object' && candidate !== null) {
    const record = candidate as Record<string, unknown>;
    return (
      normalizeThemeCandidate(record.theme) ??
      normalizeThemeCandidate(record.darkMode) ??
      normalizeThemeCandidate(record.dark_mode) ??
      normalizeThemeCandidate(record.value) ??
      (typeof record.state === 'object' && record.state !== null
        ? normalizeThemeCandidate((record.state as Record<string, unknown>).theme)
        : null)
    );
  }

  return null;
};

const readThemeFromRawStorageValue = (rawValue: string | null): UITheme | null => {
  if (rawValue == null || rawValue === '') {
    return null;
  }

  const directTheme = normalizeThemeCandidate(rawValue);
  if (directTheme) {
    return directTheme;
  }

  try {
    return normalizeThemeCandidate(JSON.parse(rawValue) as unknown);
  } catch {
    return null;
  }
};

export const resolveThemeFromStorage = (storage: StorageLike | null): UITheme | null => {
  if (!storage) {
    return null;
  }

  const persistedTheme = readThemeFromRawStorageValue(storage.getItem(persistKeys.ui));
  if (persistedTheme) {
    return persistedTheme;
  }

  for (const legacyKey of LEGACY_THEME_STORAGE_KEYS) {
    const legacyTheme = readThemeFromRawStorageValue(storage.getItem(legacyKey));
    if (legacyTheme) {
      return legacyTheme;
    }
  }

  return null;
};

const saveLegacyTheme = (theme: UITheme): void => {
  if (typeof window === 'undefined') {
    return;
  }
  window.localStorage.setItem(LEGACY_THEME_STORAGE_KEYS[0], theme);
};

const createInitialState = (): UIState => {
  const storage = typeof window !== 'undefined' ? window.localStorage : null;
  return {
    sidebarOpen: true,
    theme: resolveThemeFromStorage(storage) ?? DEFAULT_THEME,
    timelineUpdateMode: 'standard',
    isLoading: false,
    error: null,
    activeSidebarCategory: null,
  };
};

export const useUIStore = create<UIStore>()(
  withPersist<UIStore>(
    (set) => ({
      ...createInitialState(),
      toggleSidebar: () => set((state) => ({ sidebarOpen: !state.sidebarOpen })),

      setSidebarOpen: (open: boolean) => set({ sidebarOpen: open }),

      setTheme: (theme: UITheme) =>
        set((state) => {
          if (state.theme === theme) {
            return state;
          }
          saveLegacyTheme(theme);
          return { theme };
        }),

      setTimelineUpdateMode: (timelineUpdateMode: TimelineUpdateMode) =>
        set((state) => {
          if (state.timelineUpdateMode === timelineUpdateMode) {
            return state;
          }
          return { timelineUpdateMode };
        }),

      setLoading: (isLoading: boolean) => set({ isLoading }),

      setError: (error: string | null) => set({ error }),

      clearError: () => set({ error: null }),

      setActiveSidebarCategory: (category: SidebarCategory | null) =>
        set((state) =>
          state.activeSidebarCategory === category ? state : { activeSidebarCategory: category },
        ),

      resetActiveSidebarCategory: () =>
        set((state) =>
          state.activeSidebarCategory === null ? state : { activeSidebarCategory: null },
        ),
    }),
    createUiPersistConfig<UIStore>(),
  ),
);
