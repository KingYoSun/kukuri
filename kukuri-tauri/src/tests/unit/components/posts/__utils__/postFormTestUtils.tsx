import type { ReactElement, ReactNode } from 'react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render } from '@testing-library/react';
import { vi } from 'vitest';

import { useAuthStore } from '@/stores';
import { TauriApi } from '@/lib/api/tauri';
import { toast } from 'sonner';

vi.mock('@/stores', () => ({
  useAuthStore: vi.fn(() => ({
    currentUser: null,
  })),
  useBookmarkStore: vi.fn(() => ({
    bookmarks: [],
    fetchBookmarks: vi.fn(),
    addBookmark: vi.fn(),
    removeBookmark: vi.fn(),
    isBookmarked: vi.fn(() => false),
  })),
}));

vi.mock('@/lib/api/tauri', () => ({
  TauriApi: {
    createPost: vi.fn(),
  },
}));

vi.mock('sonner', () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
  },
}));

export const mockUseAuthStore = vi.mocked(useAuthStore);
export const mockTauriApi = vi.mocked(TauriApi);
export const mockToast = vi.mocked(toast);

export const createPostFormRenderer = () => {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });

  const wrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );

  return {
    renderWithQueryClient: (ui: ReactElement) => render(ui, { wrapper }),
    reset: () => queryClient.clear(),
  };
};

export interface MockProfile {
  pubkey: string;
  npub: string;
  name: string;
  displayName: string;
  picture: string;
}

export const createMockProfile = (overrides: Partial<MockProfile> = {}): MockProfile => ({
  pubkey: 'test-pubkey',
  npub: 'npub1test',
  name: 'Test User',
  displayName: 'Test Display Name',
  picture: 'https://example.com/avatar.jpg',
  ...overrides,
});
