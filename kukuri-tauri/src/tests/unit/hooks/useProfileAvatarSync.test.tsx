import { renderHook, act, waitFor } from '@testing-library/react';
import { vi, describe, beforeEach, it, expect } from 'vitest';

import { useProfileAvatarSync } from '@/hooks/useProfileAvatarSync';
import { useAuthStore } from '@/stores/authStore';
import { TauriApi } from '@/lib/api/tauri';
import { vi, describe, beforeEach, it, expect, beforeAll } from 'vitest';

vi.mock('@/stores/authStore');

vi.mock('@/lib/api/tauri', () => ({
  TauriApi: {
    profileAvatarSync: vi.fn(),
  },
}));

vi.mock('@/serviceWorker/profileAvatarSyncBridge', () => ({
  enqueueProfileAvatarSyncJob: vi.fn().mockResolvedValue(null),
  registerProfileAvatarSyncWorker: vi.fn().mockResolvedValue(null),
  PROFILE_AVATAR_SYNC_CHANNEL: 'profile-avatar-sync',
}));

const mockUseAuthStore = useAuthStore as unknown as vi.Mock;
const mockProfileAvatarSync = TauriApi.profileAvatarSync as unknown as vi.Mock;

describe('useProfileAvatarSync', () => {
  const updateUser = vi.fn();

  beforeAll(() => {
    class StubBroadcastChannel {
      name: string;
      constructor(name: string) {
        this.name = name;
      }
      postMessage = vi.fn();
      addEventListener = vi.fn();
      removeEventListener = vi.fn();
      close = vi.fn();
    }
    Object.defineProperty(global, 'BroadcastChannel', {
      value: StubBroadcastChannel,
      configurable: true,
    });
  });

  beforeEach(() => {
    vi.clearAllMocks();
    mockUseAuthStore.mockReturnValue({
      currentUser: {
        npub: 'npub1example',
        avatar: {
          docVersion: 3,
        },
      },
      updateUser,
    });
    mockProfileAvatarSync.mockResolvedValue({
      npub: 'npub1example',
      currentVersion: 4,
      updated: true,
      avatar: {
        npub: 'npub1example',
        blob_hash: 'hash123',
        format: 'image/png',
        size_bytes: 4,
        access_level: 'contacts_only',
        share_ticket: 'ticket-1',
        doc_version: 4,
        updated_at: '2025-11-09T00:00:00Z',
        content_sha256: 'abcd',
        data_base64: 'AQIDBA==',
      },
    });
  });

  it('syncNow forces a refresh when requested', async () => {
    const { result } = renderHook(() => useProfileAvatarSync({ autoStart: false }));

    await act(async () => {
      await result.current.syncNow({ force: true });
    });

    await waitFor(() => {
      expect(mockProfileAvatarSync).toHaveBeenCalledWith({
        npub: 'npub1example',
        knownDocVersion: null,
      });
    });

    expect(updateUser).toHaveBeenCalledWith(
      expect.objectContaining({
        avatar: expect.objectContaining({
          blobHash: 'hash123',
          docVersion: 4,
        }),
        picture: expect.stringContaining('data:image/png;base64,'),
      }),
    );
  });
});
