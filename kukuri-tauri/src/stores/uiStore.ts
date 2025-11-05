import { create } from 'zustand';

export type SidebarCategory = 'topics' | 'search' | 'trending' | 'following';

interface UIState {
  sidebarOpen: boolean;
  theme: 'light' | 'dark' | 'system';
  isLoading: boolean;
  error: string | null;
  activeSidebarCategory: SidebarCategory | null;
}

interface UIStore extends UIState {
  toggleSidebar: () => void;
  setSidebarOpen: (open: boolean) => void;
  setTheme: (theme: UIState['theme']) => void;
  setLoading: (isLoading: boolean) => void;
  setError: (error: string | null) => void;
  clearError: () => void;
  setActiveSidebarCategory: (category: SidebarCategory | null) => void;
  resetActiveSidebarCategory: () => void;
}

export const useUIStore = create<UIStore>()((set) => ({
  sidebarOpen: true,
  theme: 'system',
  isLoading: false,
  error: null,
  activeSidebarCategory: null,

  toggleSidebar: () => set((state) => ({ sidebarOpen: !state.sidebarOpen })),

  setSidebarOpen: (open: boolean) => set({ sidebarOpen: open }),

  setTheme: (theme: UIState['theme']) => set({ theme }),

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
}));
