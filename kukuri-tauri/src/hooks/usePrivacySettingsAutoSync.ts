import { useCallback, useEffect, useRef } from 'react';
import { useAuthStore } from '@/stores/authStore';
import { useOfflineStore } from '@/stores/offlineStore';
import { usePrivacySettingsStore } from '@/stores/privacySettingsStore';
import { syncPrivacySettings } from '@/lib/settings/privacySettingsSync';
import { errorHandler } from '@/lib/errorHandler';

export function usePrivacySettingsAutoSync() {
  const currentUser = useAuthStore((state) => state.currentUser);
  const isOnline = useOfflineStore((state) => state.isOnline);
  const hasPendingSync = usePrivacySettingsStore((state) => state.hasPendingSync);
  const inFlightRef = useRef(false);
  const previousUserNpubRef = useRef<string | null>(null);

  const syncPendingPrivacyIfNeeded = useCallback(async () => {
    const latestUser = useAuthStore.getState().currentUser;
    const latestOffline = useOfflineStore.getState();
    const latestPrivacy = usePrivacySettingsStore.getState();

    if (
      inFlightRef.current ||
      !latestUser ||
      !latestOffline.isOnline ||
      !latestPrivacy.hasPendingSync
    ) {
      return;
    }

    inFlightRef.current = true;
    try {
      await syncPrivacySettings({
        npub: latestUser.npub,
        publicProfile: latestPrivacy.publicProfile,
        showOnlineStatus: latestPrivacy.showOnlineStatus,
      });
      latestPrivacy.markSyncSuccess();
      useAuthStore.getState().updateUser({
        publicProfile: latestPrivacy.publicProfile,
        showOnlineStatus: latestPrivacy.showOnlineStatus,
      });
    } catch (error) {
      latestPrivacy.markSyncFailure(error instanceof Error ? error.message : null);
      errorHandler.log('Privacy settings auto-sync failed', error, {
        context: 'usePrivacySettingsAutoSync',
        metadata: {
          npub: latestUser.npub,
        },
      });
    } finally {
      inFlightRef.current = false;
    }
  }, []);

  useEffect(() => {
    void syncPendingPrivacyIfNeeded();
    const handleOnline = () => {
      void syncPendingPrivacyIfNeeded();
    };

    window.addEventListener('online', handleOnline);
    return () => {
      window.removeEventListener('online', handleOnline);
    };
  }, [syncPendingPrivacyIfNeeded]);

  useEffect(() => {
    const latestUserNpub = currentUser?.npub ?? null;
    const previousUserNpub = previousUserNpubRef.current;
    previousUserNpubRef.current = latestUserNpub;

    if (latestUserNpub === null || previousUserNpub === latestUserNpub) {
      return;
    }

    if (!isOnline || !hasPendingSync) {
      return;
    }

    void syncPendingPrivacyIfNeeded();
  }, [currentUser?.npub, hasPendingSync, isOnline, syncPendingPrivacyIfNeeded]);
}
