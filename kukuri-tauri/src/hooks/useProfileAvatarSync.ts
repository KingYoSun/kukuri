import { useCallback, useEffect, useMemo, useRef, useState } from 'react';

import { TauriApi } from '@/lib/api/tauri';
import { errorHandler } from '@/lib/errorHandler';
import { buildAvatarDataUrl, buildUserAvatarMetadataFromFetch } from '@/lib/profile/avatar';
import { useAuthStore } from '@/stores/authStore';
import {
  enqueueProfileAvatarSyncJob,
  PROFILE_AVATAR_SYNC_CHANNEL,
  registerProfileAvatarSyncWorker,
  type ProfileAvatarSyncJobPayload,
} from '@/serviceWorker/profileAvatarSyncBridge';

interface UseProfileAvatarSyncOptions {
  autoStart?: boolean;
  intervalMs?: number;
}

interface SyncOptions {
  force?: boolean;
}

interface ProfileAvatarSyncResult {
  isSyncing: boolean;
  error: string | null;
  lastSyncedAt: Date | null;
  syncNow: (options?: SyncOptions) => Promise<void>;
}

const DEFAULT_INTERVAL_MS = 5 * 60 * 1000; // 5 minutes

type ProfileAvatarSyncWorkerMessage =
  | {
      type: 'profile-avatar-sync:process';
      payload: {
        jobId: string;
        npub: string;
        knownDocVersion: number | null;
        force?: boolean;
      };
    }
  | {
      type: 'profile-avatar-sync:complete';
      payload: { jobId: string; success: boolean };
    };

export function useProfileAvatarSync(
  options: UseProfileAvatarSyncOptions = {},
): ProfileAvatarSyncResult {
  const { currentUser, updateUser } = useAuthStore();
  const [isSyncing, setIsSyncing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [lastSyncedAt, setLastSyncedAt] = useState<Date | null>(null);
  const lastSyncRequest = useRef<Promise<void> | null>(null);
  const workerChannelRef = useRef<BroadcastChannel | null>(null);

  const intervalMs = options.intervalMs ?? DEFAULT_INTERVAL_MS;
  const npub = currentUser?.npub;
  const currentDocVersion = currentUser?.avatar?.docVersion ?? null;

  const syncNow = useCallback(
    async (syncOptions?: SyncOptions) => {
      if (!npub) {
        return;
      }

      if (lastSyncRequest.current) {
        try {
          await lastSyncRequest.current;
        } catch {
          // ignore previous failure
        }
      }

      const request = (async () => {
        setIsSyncing(true);
        try {
          const response = await TauriApi.profileAvatarSync({
            npub,
            knownDocVersion: syncOptions?.force ? null : currentDocVersion,
          });

          if (response.updated && response.avatar) {
            const metadata = buildUserAvatarMetadataFromFetch(npub, response.avatar);
            const picture = buildAvatarDataUrl(response.avatar.format, response.avatar.data_base64);
            updateUser({
              avatar: metadata,
              picture,
            });
          }
          setLastSyncedAt(new Date());
          setError(null);
        } catch (err) {
          errorHandler.log('ProfileAvatarSync.failed', err, {
            context: 'useProfileAvatarSync.syncNow',
            metadata: { npub },
          });
          setError('プロフィール画像の同期に失敗しました');
        } finally {
          setIsSyncing(false);
        }
      })();

      lastSyncRequest.current = request;
      await request;
    },
    [currentDocVersion, npub, updateUser],
  );

  const autoStart = options.autoStart ?? true;

  useEffect(() => {
    if (typeof window === 'undefined') {
      return;
    }
    void registerProfileAvatarSyncWorker();
  }, []);

  useEffect(() => {
    if (!npub || typeof window === 'undefined' || typeof BroadcastChannel === 'undefined') {
      return;
    }

    const channel = new BroadcastChannel(PROFILE_AVATAR_SYNC_CHANNEL);
    workerChannelRef.current = channel;

    const handleMessage = (event: MessageEvent<ProfileAvatarSyncWorkerMessage>) => {
      const message = event.data;
      if (!message || message.type !== 'profile-avatar-sync:process') {
        return;
      }

      const job = message.payload;
      if (job.npub && job.npub !== npub) {
        return;
      }

      const run = async () => {
        try {
          await syncNow({ force: job.force ?? job.knownDocVersion === null });
          channel.postMessage({
            type: 'profile-avatar-sync:complete',
            payload: { jobId: job.jobId, success: true },
          } satisfies ProfileAvatarSyncWorkerMessage);
        } catch {
          channel.postMessage({
            type: 'profile-avatar-sync:complete',
            payload: { jobId: job.jobId, success: false },
          } satisfies ProfileAvatarSyncWorkerMessage);
        }
      };

      void run();
    };

    channel.addEventListener('message', handleMessage);

    return () => {
      channel.removeEventListener('message', handleMessage);
      channel.close();
      workerChannelRef.current = null;
    };
  }, [npub, syncNow]);

  useEffect(() => {
    if (!autoStart || !npub) {
      return;
    }

    let disposed = false;
    let intervalId: number | null = null;

    const scheduleJob = async (force: boolean) => {
      const payload: ProfileAvatarSyncJobPayload = {
        npub,
        knownDocVersion: force ? null : currentDocVersion,
        source: force ? 'useProfileAvatarSync:bootstrap' : 'useProfileAvatarSync:interval',
        force,
      };
      const jobId = await enqueueProfileAvatarSyncJob(payload);
      if (!jobId) {
        await syncNow({ force });
      }
    };

    const start = async () => {
      await scheduleJob(true);
      if (disposed) {
        return;
      }
      intervalId = window.setInterval(() => {
        void scheduleJob(false);
      }, intervalMs);
    };

    void start();

    return () => {
      disposed = true;
      if (intervalId !== null) {
        window.clearInterval(intervalId);
      }
    };
  }, [autoStart, intervalMs, npub, syncNow]);

  return useMemo(
    () => ({
      isSyncing,
      error,
      lastSyncedAt,
      syncNow,
    }),
    [isSyncing, error, lastSyncedAt, syncNow],
  );
}
