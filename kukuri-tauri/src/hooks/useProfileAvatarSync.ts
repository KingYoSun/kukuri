import { useCallback, useEffect, useMemo, useRef, useState } from 'react';

import {
  TauriApi,
  type ProfileAvatarSyncResult as ProfileAvatarSyncApiResult,
} from '@/lib/api/tauri';
import { offlineApi } from '@/api/offline';
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
  jobId?: string;
  source?: string;
  requestedAt?: string;
  retryCount?: number;
  knownDocVersion?: number | null;
}

interface UseProfileAvatarSyncResult {
  isSyncing: boolean;
  error: string | null;
  lastSyncedAt: Date | null;
  syncNow: (options?: SyncOptions) => Promise<ProfileAvatarSyncApiResult | undefined>;
}

const DEFAULT_INTERVAL_MS = 5 * 60 * 1000; // 5 minutes
const WORKER_SOURCE_PREFIX = 'profile-avatar-sync-worker';
const MANUAL_SOURCE = 'useProfileAvatarSync:manual';

type ProfileAvatarSyncWorkerMessage =
  | {
      type: 'profile-avatar-sync:process';
      payload: ProfileAvatarSyncJobPayload;
    }
  | {
      type: 'profile-avatar-sync:complete';
      payload: { jobId: string; success: boolean };
    };

function shouldLogToSyncQueue(source?: string): boolean {
  return Boolean(source && source.startsWith(WORKER_SOURCE_PREFIX));
}

async function logWorkerSyncQueueEntry(
  job: ProfileAvatarSyncJobPayload,
  success: boolean,
  result?: ProfileAvatarSyncApiResult,
  err?: unknown,
) {
  if (!shouldLogToSyncQueue(job.source)) {
    return;
  }

  const errorMessage =
    success || !err
      ? null
      : err instanceof Error
        ? err.message
        : typeof err === 'string'
          ? err
          : 'unknown-error';

  try {
    await offlineApi.addToSyncQueue({
      action_type: 'profile_avatar_sync',
      payload: {
        jobId: job.jobId,
        npub: job.npub,
        source: job.source,
        requestedAt: job.requestedAt,
        retryCount: job.retryCount ?? 0,
        knownDocVersion: job.knownDocVersion ?? null,
        success,
        updated: result?.updated ?? false,
        currentVersion: result?.currentVersion ?? null,
        error: errorMessage,
      },
      priority: 2,
    });
  } catch (queueError) {
    errorHandler.log('ProfileAvatarSync.syncQueueLogFailed', queueError, {
      context: 'useProfileAvatarSync.logWorkerSync',
      metadata: {
        jobId: job.jobId,
        source: job.source,
      },
    });
  }
}

export function useProfileAvatarSync(
  options: UseProfileAvatarSyncOptions = {},
): UseProfileAvatarSyncResult {
  const { currentUser, updateUser } = useAuthStore();
  const [isSyncing, setIsSyncing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [lastSyncedAt, setLastSyncedAt] = useState<Date | null>(null);
  const lastSyncRequest = useRef<Promise<ProfileAvatarSyncApiResult | undefined> | null>(null);
  const workerChannelRef = useRef<BroadcastChannel | null>(null);

  const intervalMs = options.intervalMs ?? DEFAULT_INTERVAL_MS;
  const npub = currentUser?.npub;
  const currentDocVersion = currentUser?.avatar?.docVersion ?? null;

  const syncNow = useCallback(
    async (syncOptions?: SyncOptions): Promise<ProfileAvatarSyncApiResult | undefined> => {
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
          const source = syncOptions?.source ?? MANUAL_SOURCE;
          const requestedAt = syncOptions?.requestedAt ?? new Date().toISOString();
          const retryCount = syncOptions?.retryCount ?? 0;
          const knownDocVersion =
            syncOptions?.force === true
              ? null
              : (syncOptions?.knownDocVersion ?? (syncOptions?.force ? null : currentDocVersion));
          const response = await TauriApi.profileAvatarSync({
            npub,
            knownDocVersion,
            source,
            requestedAt,
            retryCount,
            jobId: syncOptions?.jobId,
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
          return response;
        } catch (err) {
          errorHandler.log('ProfileAvatarSync.failed', err, {
            context: 'useProfileAvatarSync.syncNow',
            metadata: { npub, jobId: syncOptions?.jobId },
          });
          setError('プロフィール画像の同期に失敗しました');
          throw err;
        } finally {
          setIsSyncing(false);
        }
      })();

      lastSyncRequest.current = request;
      return await request;
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
      if (!job.jobId) {
        errorHandler.log('ProfileAvatarSync.missingJobId', new Error('jobId missing'), {
          context: 'useProfileAvatarSync.handleMessage',
          metadata: { npub, payload: job },
        });
        return;
      }
      const jobId = job.jobId;

      const run = async () => {
        const syncPayload: SyncOptions = {
          force: job.force ?? job.knownDocVersion === null,
          jobId,
          source: job.source,
          requestedAt: job.requestedAt,
          retryCount: job.retryCount,
          knownDocVersion: job.knownDocVersion,
        };
        try {
          const result = await syncNow(syncPayload);
          await logWorkerSyncQueueEntry(job, true, result);
          channel.postMessage({
            type: 'profile-avatar-sync:complete',
            payload: { jobId, success: true },
          } satisfies ProfileAvatarSyncWorkerMessage);
        } catch (err) {
          await logWorkerSyncQueueEntry(job, false, undefined, err);
          channel.postMessage({
            type: 'profile-avatar-sync:complete',
            payload: { jobId, success: false },
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
        source: force ? `${WORKER_SOURCE_PREFIX}:bootstrap` : `${WORKER_SOURCE_PREFIX}:interval`,
        force,
      };
      const jobId = await enqueueProfileAvatarSyncJob(payload);
      if (!jobId) {
        await syncNow({
          force,
          source: force ? `${MANUAL_SOURCE}:bootstrap` : `${MANUAL_SOURCE}:interval-fallback`,
        });
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
