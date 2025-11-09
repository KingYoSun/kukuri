import { create } from 'zustand';

import { withPersist } from './utils/persistHelpers';
import { persistKeys } from './config/persist';
import type { User } from './types';

interface PrivacySettingsState {
  publicProfile: boolean;
  showOnlineStatus: boolean;
}

interface PrivacySettingsStore extends PrivacySettingsState {
  setPublicProfile: (value: boolean) => void;
  setShowOnlineStatus: (value: boolean) => void;
  hydrateFromUser: (user: Pick<User, 'publicProfile' | 'showOnlineStatus'> | null) => void;
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
      hydrateFromUser: (user) =>
        set((state) => {
          if (!user) {
            return {
              publicProfile: state.publicProfile,
              showOnlineStatus: state.showOnlineStatus,
            };
          }
          return {
            publicProfile:
              typeof user.publicProfile === 'boolean' ? user.publicProfile : state.publicProfile,
            showOnlineStatus:
              typeof user.showOnlineStatus === 'boolean'
                ? user.showOnlineStatus
                : state.showOnlineStatus,
          };
        }),
      reset: () =>
        set((state) => ({
          ...state,
          ...createInitialState(),
        })),
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
