import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import type { ReactNode } from 'react';
import { vi } from 'vitest';

export const mockNavigate = vi.fn();

vi.mock('@tanstack/react-router', () => ({
  useNavigate: () => mockNavigate,
}));

vi.mock('sonner', () => ({
  toast: {
    error: vi.fn(),
    success: vi.fn(),
  },
}));

vi.mock('@/stores/authStore');
vi.mock('@/lib/api/nostr');
vi.mock('@/lib/errorHandler', () => ({
  errorHandler: {
    log: vi.fn(),
  },
}));

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn(),
}));

vi.mock('@tauri-apps/plugin-fs', () => ({
  readFile: vi.fn(),
}));

vi.mock('@/lib/api/tauri', () => ({
  TauriApi: {
    uploadProfileAvatar: vi.fn(),
    fetchProfileAvatar: vi.fn(),
    updatePrivacySettings: vi.fn(),
    profileAvatarSync: vi.fn(),
  },
}));

export const createQueryWrapper = () => {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });

  return ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );
};

export const initializeTauriMocks = async () => {
  const dialogModule = await import('@tauri-apps/plugin-dialog');
  const fsModule = await import('@tauri-apps/plugin-fs');
  const tauriModule = await import('@/lib/api/tauri');

  tauriModule.TauriApi.updatePrivacySettings = vi.fn();
  tauriModule.TauriApi.profileAvatarSync = vi.fn();

  return {
    mockOpen: dialogModule.open as unknown as ReturnType<typeof vi.fn>,
    mockReadFile: fsModule.readFile as unknown as ReturnType<typeof vi.fn>,
    mockUploadProfileAvatar: tauriModule.TauriApi.uploadProfileAvatar as unknown as ReturnType<
      typeof vi.fn
    >,
    mockFetchProfileAvatar: tauriModule.TauriApi.fetchProfileAvatar as unknown as ReturnType<
      typeof vi.fn
    >,
    mockUpdatePrivacySettings: tauriModule.TauriApi.updatePrivacySettings as unknown as ReturnType<
      typeof vi.fn
    >,
    mockProfileAvatarSync: tauriModule.TauriApi.profileAvatarSync as unknown as ReturnType<
      typeof vi.fn
    >,
  };
};

export const stubObjectUrl = () => {
  const originalCreateObjectURL = global.URL.createObjectURL;
  const originalRevokeObjectURL = global.URL.revokeObjectURL;

  return {
    setup: () => {
      global.URL.createObjectURL = vi.fn(() => 'blob:profile-test');
      global.URL.revokeObjectURL = vi.fn();
    },
    restore: () => {
      global.URL.createObjectURL = originalCreateObjectURL;
      global.URL.revokeObjectURL = originalRevokeObjectURL;
    },
  };
};
