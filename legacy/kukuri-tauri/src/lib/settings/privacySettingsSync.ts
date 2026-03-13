import { updateNostrMetadata } from '@/lib/api/nostr';
import { TauriApi } from '@/lib/api/tauri';
import { errorHandler } from '@/lib/errorHandler';

export interface PrivacySettingsSyncPayload {
  npub: string;
  publicProfile: boolean;
  showOnlineStatus: boolean;
}

export const syncPrivacySettings = async (payload: PrivacySettingsSyncPayload): Promise<void> => {
  await TauriApi.updatePrivacySettings({
    npub: payload.npub,
    publicProfile: payload.publicProfile,
    showOnlineStatus: payload.showOnlineStatus,
  });

  try {
    await updateNostrMetadata({
      kukuri_privacy: {
        public_profile: payload.publicProfile,
        show_online_status: payload.showOnlineStatus,
      },
    });
  } catch (error) {
    errorHandler.log('syncPrivacySettings.updateNostrMetadataSkipped', error, {
      context: 'privacySettingsSync.syncPrivacySettings',
      metadata: {
        npub: payload.npub,
      },
    });
  }
};
