import { renderHook, act, waitFor } from '@testing-library/react';
import { vi, describe, beforeEach, it, expect, beforeAll } from 'vitest';

import { useProfileAvatarSync } from '@/hooks/useProfileAvatarSync';
import { useAuthStore } from '@/stores/authStore';
import { TauriApi } from '@/lib/api/tauri';
import { offlineApi } from '@/api/offline';

vi.mock('@/stores/authStore');

vi.mock('@/lib/api/tauri', () => ({
  TauriApi: {
    profileAvatarSync: vi.fn(),
  },
}));

vi.mock('@/api/offline', () => ({
  offlineApi: {
    addToSyncQueue: vi.fn(),
  },
}));

vi.mock('@/serviceWorker/profileAvatarSyncBridge', () => ({
  enqueueProfileAvatarSyncJob: vi.fn().mockResolvedValue(null),
  registerProfileAvatarSyncWorker: vi.fn().mockResolvedValue(null),
  PROFILE_AVATAR_SYNC_CHANNEL: 'profile-avatar-sync',
}));

const stubChannels: StubBroadcastChannel[] = [];

class StubBroadcastChannel {
  name: string;
  listeners: Array<(event: MessageEvent<any>) => void> = [];
  constructor(name: string) {
    this.name = name;
    stubChannels.push(this);
  }
  postMessage = vi.fn();
  addEventListener = vi.fn((type: string, listener: (event: MessageEvent<any>) => void) => {
    if (type === 'message') {
      this.listeners.push(listener);
    }
  });
  removeEventListener = vi.fn((type: string, listener: (event: MessageEvent<any>) => void) => {
    if (type === 'message') {
      this.listeners = this.listeners.filter((cb) => cb !== listener);
    }
  });
  close = vi.fn();
  emit(data: any) {
    for (const listener of this.listeners) {
      listener({ data } as MessageEvent<any>);
    }
  }
}

const mockUseAuthStore = useAuthStore as unknown as vi.Mock;
const mockProfileAvatarSync = TauriApi.profileAvatarSync as unknown as vi.Mock;
const mockAddToSyncQueue = offlineApi.addToSyncQueue as unknown as vi.Mock;

describe('useProfileAvatarSync', () => {
  const updateUser = vi.fn();

  beforeAll(() => {
    Object.defineProperty(global, 'BroadcastChannel', {
      value: StubBroadcastChannel,
      configurable: true,
    });
  });

  beforeEach(() => {
    vi.clearAllMocks();
    stubChannels.length = 0;
    mockUseAuthStore.mockReturnValue({
      currentUser: {
        npub: 'npub1example',
        avatar: {
          docVersion: 3,
        },
      },
      updateUser,
    });
    mockAddToSyncQueue.mockResolvedValue(1);
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
      expect(mockProfileAvatarSync).toHaveBeenCalledWith(
        expect.objectContaining({
          npub: 'npub1example',
          knownDocVersion: null,
          source: 'useProfileAvatarSync:manual',
          retryCount: 0,
          requestedAt: expect.any(String),
        }),
      );
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

  it('records Service Worker jobs in sync_queue and posts completion events', async () => {
    renderHook(() => useProfileAvatarSync({ autoStart: false }));
    await waitFor(() => {
      expect(stubChannels.length).toBeGreaterThan(0);
    });
    const channel = stubChannels[0];
    const jobPayload = {
      jobId: 'job-1',
      npub: 'npub1example',
      knownDocVersion: 2,
      source: 'profile-avatar-sync-worker:interval',
      requestedAt: '2025-11-12T00:00:00Z',
      retryCount: 1,
    };

    await act(async () => {
      channel?.emit({
        type: 'profile-avatar-sync:process',
        payload: jobPayload,
      });
    });

    await waitFor(() => {
      expect(mockProfileAvatarSync).toHaveBeenCalledWith(
        expect.objectContaining({
          jobId: 'job-1',
          source: 'profile-avatar-sync-worker:interval',
          requestedAt: '2025-11-12T00:00:00Z',
          retryCount: 1,
        }),
      );
    });

    await waitFor(() => {
      expect(mockAddToSyncQueue).toHaveBeenCalledWith(
        expect.objectContaining({
          action_type: 'profile_avatar_sync',
          payload: expect.objectContaining({
            jobId: 'job-1',
            source: 'profile-avatar-sync-worker:interval',
            success: true,
          }),
        }),
      );
    });

    expect(channel?.postMessage).toHaveBeenCalledWith(
      expect.objectContaining({
        type: 'profile-avatar-sync:complete',
        payload: { jobId: 'job-1', success: true },
      }),
    );
  });
});
