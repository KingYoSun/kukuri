import { create } from 'zustand';

import { withPersist } from './utils/persistHelpers';
import { persistKeys } from './config/persist';

interface PrivacySettingsState {
  publicProfile: boolean;
  showOnlineStatus: boolean;
}

interface PrivacySettingsStore extends PrivacySettingsState {
  setPublicProfile: (value: boolean) => void;
  setShowOnlineStatus: (value: boolean) => void;
  reset: () => void;
}

const createInitialState = (): PrivacySettingsState => ({
  publicProfile: true,
  showOnlineStatus: false,
});

export const getPrivacySettingsInitialState = () => createInitialState();

export const usePrivacySettingsStore = create<PrivacySettingsStore>()(
  withPersist<PrivacySettingsStore>(
    (set) => ({
      ...createInitialState(),
      setPublicProfile: (value) => set({ publicProfile: value }),
      setShowOnlineStatus: (value) => set({ showOnlineStatus: value }),
      reset: () => set(createInitialState()),
    }),
    {
      name: persistKeys.privacy,
      partialize: ({ publicProfile, showOnlineStatus }) => ({
        publicProfile,
        showOnlineStatus,
      }),
    },
  ),
);
