import { useCallback, useEffect, useMemo, useRef, useState } from 'react';

import { TauriApi } from '@/lib/api/tauri';
import { errorHandler } from '@/lib/errorHandler';
import { buildAvatarDataUrl, buildUserAvatarMetadataFromFetch } from '@/lib/profile/avatar';
import { useAuthStore } from '@/stores/authStore';

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

export function useProfileAvatarSync(
  options: UseProfileAvatarSyncOptions = {},
): ProfileAvatarSyncResult {
  const { currentUser, updateUser } = useAuthStore();
  const [isSyncing, setIsSyncing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [lastSyncedAt, setLastSyncedAt] = useState<Date | null>(null);
  const lastSyncRequest = useRef<Promise<void> | null>(null);

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
    if (!autoStart || !npub) {
      return;
    }

    let disposed = false;
    let intervalId: number | null = null;

    const start = async () => {
      await syncNow({ force: true });
      if (disposed) {
        return;
      }
      intervalId = window.setInterval(() => {
        void syncNow();
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
