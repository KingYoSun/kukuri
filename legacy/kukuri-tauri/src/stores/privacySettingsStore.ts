import { create } from 'zustand';

import { withPersist } from './utils/persistHelpers';
import { persistKeys } from './config/persist';
import type { User } from './types';

interface PrivacySettingsState {
  publicProfile: boolean;
  showOnlineStatus: boolean;
  ownerNpub: string | null;
  hasPendingSync: boolean;
  lastSyncedAt: number | null;
  lastSyncError: string | null;
  updatedAt: number | null;
}

interface PrivacySettingsStore extends PrivacySettingsState {
  setPublicProfile: (value: boolean) => void;
  setShowOnlineStatus: (value: boolean) => void;
  applyLocalChange: (payload: {
    npub?: string;
    publicProfile?: boolean;
    showOnlineStatus?: boolean;
  }) => void;
  markSyncSuccess: () => void;
  markSyncFailure: (reason?: string | null) => void;
  hydrateFromUser: (user: Pick<User, 'npub' | 'publicProfile' | 'showOnlineStatus'> | null) => void;
  reset: () => void;
}

const createInitialState = (): PrivacySettingsState => ({
  publicProfile: true,
  showOnlineStatus: false,
  ownerNpub: null,
  hasPendingSync: false,
  lastSyncedAt: null,
  lastSyncError: null,
  updatedAt: null,
});

export const getPrivacySettingsInitialState = () => createInitialState();
const nextUpdatedAt = () => Date.now();

export const usePrivacySettingsStore = create<PrivacySettingsStore>()(
  withPersist<PrivacySettingsStore>(
    (set) => ({
      ...createInitialState(),
      setPublicProfile: (value) =>
        set(() => ({
          publicProfile: value,
          hasPendingSync: true,
          lastSyncError: null,
          updatedAt: nextUpdatedAt(),
        })),
      setShowOnlineStatus: (value) =>
        set(() => ({
          showOnlineStatus: value,
          hasPendingSync: true,
          lastSyncError: null,
          updatedAt: nextUpdatedAt(),
        })),
      applyLocalChange: (payload) =>
        set((state) => ({
          ownerNpub: payload.npub ?? state.ownerNpub,
          publicProfile:
            typeof payload.publicProfile === 'boolean'
              ? payload.publicProfile
              : state.publicProfile,
          showOnlineStatus:
            typeof payload.showOnlineStatus === 'boolean'
              ? payload.showOnlineStatus
              : state.showOnlineStatus,
          hasPendingSync: true,
          lastSyncError: null,
          updatedAt: nextUpdatedAt(),
        })),
      markSyncSuccess: () =>
        set((state) => ({
          hasPendingSync: false,
          lastSyncedAt: Math.max(state.lastSyncedAt ?? 0, nextUpdatedAt()),
          lastSyncError: null,
        })),
      markSyncFailure: (reason) =>
        set(() => ({
          hasPendingSync: true,
          lastSyncError: reason ?? 'sync_failed',
        })),
      hydrateFromUser: (user) =>
        set((state) => {
          if (!user) {
            return state;
          }
          const isSameUser = state.ownerNpub === user.npub;
          if (isSameUser && state.hasPendingSync) {
            return {
              ownerNpub: user.npub,
            };
          }
          return {
            publicProfile:
              typeof user.publicProfile === 'boolean' ? user.publicProfile : state.publicProfile,
            showOnlineStatus:
              typeof user.showOnlineStatus === 'boolean'
                ? user.showOnlineStatus
                : state.showOnlineStatus,
            ownerNpub: user.npub,
            hasPendingSync: false,
            lastSyncError: null,
          };
        }),
      reset: () => set(() => createInitialState()),
    }),
    {
      name: persistKeys.privacy,
      partialize: ({
        publicProfile,
        showOnlineStatus,
        ownerNpub,
        hasPendingSync,
        lastSyncedAt,
        lastSyncError,
        updatedAt,
      }) => ({
        publicProfile,
        showOnlineStatus,
        ownerNpub,
        hasPendingSync,
        lastSyncedAt,
        lastSyncError,
        updatedAt,
      }),
    },
  ),
);
