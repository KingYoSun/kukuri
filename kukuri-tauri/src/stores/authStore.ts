import { create } from 'zustand';

import type { AuthState, User } from './types';
import { TauriApi } from '@/lib/api/tauri';
import { initializeNostr, disconnectNostr, getRelayStatus, type RelayInfo } from '@/lib/api/nostr';
import { SecureStorageApi, type AccountMetadata } from '@/lib/api/secureStorage';
import { errorHandler } from '@/lib/errorHandler';
import { useTopicStore } from './topicStore';
import { usePrivacySettingsStore } from './privacySettingsStore';
import { withPersist } from './utils/persistHelpers';
import { createAuthPersistConfig } from './config/persist';
import { DEFAULT_PUBLIC_TOPIC_ID } from '@/constants/topics';
import { buildAvatarDataUrl, buildUserAvatarMetadataFromFetch } from '@/lib/profile/avatar';
import { setE2EAuthDebug } from '@/lib/utils/e2eDebug';

const DEFAULT_RELAY_STATUS_INTERVAL = 30_000;
const RELAY_STATUS_BACKOFF_SEQUENCE = [120_000, 300_000, 600_000];

type FallbackAccount = {
  metadata: AccountMetadata;
  nsec: string;
};

const fallbackAccounts = new Map<string, FallbackAccount>();

const upsertFallbackAccount = (metadata: AccountMetadata, nsec: string) => {
  fallbackAccounts.set(metadata.npub, { metadata: { ...metadata }, nsec });
};

const removeFallbackAccount = (npub: string) => {
  fallbackAccounts.delete(npub);
};

export const clearFallbackAccounts = () => {
  fallbackAccounts.clear();
};

export const listFallbackAccountMetadata = (): AccountMetadata[] =>
  Array.from(fallbackAccounts.values()).map((item) => item.metadata);

const getFallbackNsec = (npub: string): string | null => {
  const entry = fallbackAccounts.get(npub);
  return entry?.nsec ?? null;
};

const updateFallbackAccountMetadata = (npub: string, update: Partial<AccountMetadata>) => {
  const existing = fallbackAccounts.get(npub);
  if (!existing) {
    return;
  }
  fallbackAccounts.set(npub, {
    ...existing,
    metadata: {
      ...existing.metadata,
      ...update,
    },
  });
};

const buildAccountMetadata = (user: User, lastUsed?: string): AccountMetadata => ({
  npub: user.npub,
  pubkey: user.pubkey,
  name: user.name,
  display_name: user.displayName,
  picture: user.picture,
  last_used: lastUsed ?? new Date().toISOString(),
  public_profile: user.publicProfile,
  show_online_status: user.showOnlineStatus,
});

const toUserOverride = (metadata?: AccountMetadata): Partial<User> | undefined => {
  if (!metadata) {
    return undefined;
  }
  return {
    name: metadata.name,
    displayName: metadata.display_name || metadata.name,
    picture: metadata.picture ?? '',
    publicProfile:
      typeof metadata.public_profile === 'boolean' ? metadata.public_profile : undefined,
    showOnlineStatus:
      typeof metadata.show_online_status === 'boolean' ? metadata.show_online_status : undefined,
  };
};

const applyUserMetadataOverride = (base: User, override?: Partial<User>): User => {
  if (!override) {
    return base;
  }
  return {
    ...base,
    name: override.name ?? base.name,
    displayName: override.displayName ?? base.displayName,
    about: override.about ?? base.about,
    picture: override.picture ?? base.picture,
    nip05: override.nip05 ?? base.nip05,
    publicProfile:
      typeof override.publicProfile === 'boolean' ? override.publicProfile : base.publicProfile,
    showOnlineStatus:
      typeof override.showOnlineStatus === 'boolean'
        ? override.showOnlineStatus
        : base.showOnlineStatus,
  };
};

const nextRelayStatusBackoff = (current: number) => {
  for (const value of RELAY_STATUS_BACKOFF_SEQUENCE) {
    if (current < value) {
      return value;
    }
  }
  return RELAY_STATUS_BACKOFF_SEQUENCE[RELAY_STATUS_BACKOFF_SEQUENCE.length - 1];
};

const hydratePrivacyFromUser = (user: User | null) => {
  usePrivacySettingsStore.getState().hydrateFromUser(user);
};

interface AuthStore extends AuthState {
  relayStatus: RelayInfo[];
  relayStatusError: string | null;
  relayStatusBackoffMs: number;
  lastRelayStatusFetchedAt: number | null;
  isFetchingRelayStatus: boolean;
  accounts: AccountMetadata[];
  login: (privateKey: string, user: User) => Promise<void>;
  loginWithNsec: (
    nsec: string,
    saveToSecureStorage?: boolean,
    metadataOverride?: Partial<User>,
  ) => Promise<void>;
  generateNewKeypair: (saveToSecureStorage?: boolean) => Promise<{ nsec: string }>;
  logout: () => Promise<void>;
  updateUser: (user: Partial<User>) => void;
  updateRelayStatus: () => Promise<void>;
  setRelayStatus: (status: RelayInfo[]) => void;
  initialize: () => Promise<void>;
  switchAccount: (npub: string) => Promise<void>;
  removeAccount: (npub: string) => Promise<void>;
  loadAccounts: () => Promise<void>;
  get isLoggedIn(): boolean;
}

export const useAuthStore = create<AuthStore>()(
  withPersist<AuthStore>((set, get) => {
    const isAvatarNotFoundError = (error: unknown) => {
      if (!error) {
        return false;
      }
      const message =
        error instanceof Error ? error.message : typeof error === 'string' ? error : undefined;
      return typeof message === 'string' && message.includes('Profile avatar not found');
    };

    const fetchAndApplyAvatar = async (npub: string) => {
      try {
        const result = await TauriApi.fetchProfileAvatar(npub);
        const metadata = buildUserAvatarMetadataFromFetch(npub, result);
        const picture = buildAvatarDataUrl(result.format, result.data_base64);
        set((state) => {
          if (!state.currentUser || state.currentUser.npub !== npub) {
            return {};
          }
          return {
            currentUser: {
              ...state.currentUser,
              picture,
              avatar: metadata,
            },
          };
        });
      } catch (error) {
        if (isAvatarNotFoundError(error)) {
          return;
        }
        errorHandler.log('Failed to fetch profile avatar', error, {
          context: `AuthStore.fetchAndApplyAvatar (npub: ${npub})`,
        });
      }
    };

    const updateAuthDebug = () => {
      const state = useAuthStore.getState();
      setE2EAuthDebug({
        isAuthenticated: state.isAuthenticated,
        npub: state.currentUser?.npub ?? null,
        accounts: state.accounts?.map((account) => ({
          npub: account.npub,
          display_name: account.display_name,
        })),
      });
    };

    const bootstrapTopics = async () => {
      const topicStore = useTopicStore.getState();
      try {
        await topicStore.fetchTopics();
        const publicTopic = Array.from(topicStore.topics.values()).find(
          (topic) => topic.id === DEFAULT_PUBLIC_TOPIC_ID,
        );
        if (publicTopic) {
          await topicStore.joinTopic(DEFAULT_PUBLIC_TOPIC_ID);
          if (!topicStore.currentTopic) {
            topicStore.setCurrentTopic(publicTopic);
          }
        }
      } catch (error) {
        errorHandler.log('Failed to bootstrap topics', error, {
          context: 'AuthStore.bootstrapTopics',
        });
      }
    };

    return {
      isAuthenticated: false,
      currentUser: null,
      privateKey: null,
      relayStatus: [],
      relayStatusError: null,
      relayStatusBackoffMs: DEFAULT_RELAY_STATUS_INTERVAL,
      lastRelayStatusFetchedAt: null,
      isFetchingRelayStatus: false,
      accounts: [],

      login: async (privateKey: string, user: User) => {
        set({
          isAuthenticated: true,
          currentUser: user,
          privateKey,
        });
        updateAuthDebug();
        hydratePrivacyFromUser(user);
        try {
          await initializeNostr();
          await bootstrapTopics();
          await fetchAndApplyAvatar(user.npub);
        } catch (error) {
          errorHandler.log('Failed to initialize Nostr', error, {
            context: 'AuthStore.login',
          });
        }
      },

      loginWithNsec: async (
        nsec: string,
        saveToSecureStorage = false,
        metadataOverride?: Partial<User>,
      ) => {
        try {
          errorHandler.info('Attempting login with nsec', 'AuthStore.loginWithNsec', {
            saveToSecureStorage,
          });
          const response = await TauriApi.login({ nsec });
          const user: User = {
            id: response.public_key,
            pubkey: response.public_key,
            npub: response.npub,
            name: 'ユーザー',
            displayName: 'ユーザー',
            about: '',
            picture: '',
            nip05: '',
            publicProfile: true,
            showOnlineStatus: false,
            avatar: null,
          };
          const mergedUser = applyUserMetadataOverride(user, metadataOverride);
          const accountMetadata = buildAccountMetadata(mergedUser);

          if (saveToSecureStorage) {
            try {
              await SecureStorageApi.addAccount({
                nsec,
                name: mergedUser.name,
                display_name: mergedUser.displayName,
                picture: mergedUser.picture,
              });
            } catch (storageError) {
              errorHandler.log('Secure storage add failed (loginWithNsec)', storageError, {
                context: 'AuthStore.loginWithNsec',
              });
              upsertFallbackAccount(accountMetadata, nsec);
            }
          }
          upsertFallbackAccount(accountMetadata, nsec);

          set({
            isAuthenticated: true,
            currentUser: mergedUser,
            privateKey: nsec,
          });
          updateAuthDebug();
          hydratePrivacyFromUser(mergedUser);
          errorHandler.info('Auth state set after loginWithNsec', 'AuthStore.loginWithNsec', {
            npub: mergedUser.npub,
            saveToSecureStorage,
          });

          try {
            await initializeNostr();
          } catch (nostrError) {
            errorHandler.log('Failed to initialize Nostr', nostrError, {
              context: 'AuthStore.loginWithNsec.initializeNostr',
            });
          }
          try {
            await useAuthStore.getState().updateRelayStatus();
          } catch (relayError) {
            errorHandler.log('Failed to update relay status', relayError, {
              context: 'AuthStore.loginWithNsec.updateRelayStatus',
            });
          }
          try {
            await useAuthStore.getState().loadAccounts();
          } catch (loadError) {
            errorHandler.log('Failed to load accounts', loadError, {
              context: 'AuthStore.loginWithNsec.loadAccounts',
            });
          }

          await bootstrapTopics();
          await fetchAndApplyAvatar(response.npub);
        } catch (error) {
          errorHandler.log('Login failed', error, {
            context: 'AuthStore.loginWithNsec',
            showToast: true,
            toastTitle: 'ログインに失敗しました',
          });
          throw error;
        }
      },

      generateNewKeypair: async (saveToSecureStorage = true) => {
        try {
          const state = get();
          if (!state.isAuthenticated && state.accounts.length === 0) {
            clearFallbackAccounts();
          }
          errorHandler.info('Generating new keypair', 'AuthStore.generateNewKeypair', {
            saveToSecureStorage,
          });
          const response = await TauriApi.generateKeypair();
          const user: User = {
            id: response.public_key,
            pubkey: response.public_key,
            npub: response.npub,
            name: '新規ユーザー',
            displayName: '新規ユーザー',
            about: '',
            picture: '',
            nip05: '',
            publicProfile: true,
            showOnlineStatus: false,
            avatar: null,
          };
          const accountMetadata = buildAccountMetadata(user);

          // セキュアストレージに保存
          if (saveToSecureStorage) {
            errorHandler.info(
              'Saving new account to secure storage...',
              'AuthStore.generateNewKeypair',
            );
            try {
              await SecureStorageApi.addAccount({
                nsec: response.nsec,
                name: user.name,
                display_name: user.displayName,
                picture: user.picture,
              });
              errorHandler.info('Account saved successfully', 'AuthStore.generateNewKeypair');
            } catch (storageError) {
              errorHandler.log('Secure storage add failed (generateNewKeypair)', storageError, {
                context: 'AuthStore.generateNewKeypair',
              });
              upsertFallbackAccount(accountMetadata, response.nsec);
            }
          }
          // secure storage の成否に関わらずフォールバックにも保持してアカウント切替を安定させる
          upsertFallbackAccount(accountMetadata, response.nsec);

          set({
            isAuthenticated: true,
            currentUser: user,
            privateKey: response.nsec,
          });
          updateAuthDebug();
          hydratePrivacyFromUser(user);
          errorHandler.info(
            'Auth state set after keypair generation',
            'AuthStore.generateNewKeypair',
            {
              npub: user.npub,
              saveToSecureStorage,
            },
          );

          // Nostrクライアントを初期化
          try {
            await initializeNostr();
          } catch (nostrError) {
            errorHandler.log('Failed to initialize Nostr', nostrError, {
              context: 'AuthStore.generateNewKeypair.initializeNostr',
            });
          }
          // リレー状態を更新
          try {
            await useAuthStore.getState().updateRelayStatus();
          } catch (relayError) {
            errorHandler.log('Failed to update relay status', relayError, {
              context: 'AuthStore.generateNewKeypair.updateRelayStatus',
            });
          }
          // アカウントリストを更新
          try {
            await useAuthStore.getState().loadAccounts();
          } catch (loadError) {
            errorHandler.log('Failed to load accounts', loadError, {
              context: 'AuthStore.generateNewKeypair.loadAccounts',
            });
          }

          await bootstrapTopics();

          await fetchAndApplyAvatar(response.npub);

          return { nsec: response.nsec };
        } catch (error) {
          errorHandler.log('Keypair generation failed', error, {
            context: 'AuthStore.generateNewKeypair',
            showToast: true,
            toastTitle: 'キーペアの生成に失敗しました',
          });
          throw error;
        }
      },

      logout: async () => {
        try {
          await disconnectNostr();
        } catch (error) {
          errorHandler.log('Failed to disconnect Nostr', error, {
            context: 'AuthStore.logout',
          });
        }
        try {
          await TauriApi.logout();
        } catch (error) {
          errorHandler.log('Logout failed', error, {
            context: 'AuthStore.logout',
          });
        }
        set({
          isAuthenticated: false,
          currentUser: null,
          privateKey: null,
          relayStatus: [],
          relayStatusError: null,
          relayStatusBackoffMs: DEFAULT_RELAY_STATUS_INTERVAL,
          lastRelayStatusFetchedAt: null,
          isFetchingRelayStatus: false,
        });
        updateAuthDebug();
      },

      updateUser: (userUpdate: Partial<User>) =>
        set((state) => {
          if (!state.currentUser) {
            return { currentUser: null };
          }
          const updatedUser = {
            ...state.currentUser,
            ...userUpdate,
          };
          hydratePrivacyFromUser(updatedUser);
          updateFallbackAccountMetadata(updatedUser.npub, {
            name: updatedUser.name,
            display_name: updatedUser.displayName,
            picture: updatedUser.picture,
            public_profile: updatedUser.publicProfile,
            show_online_status: updatedUser.showOnlineStatus,
          });
          return {
            currentUser: updatedUser,
            accounts: state.accounts.map((account) =>
              account.npub === updatedUser.npub
                ? {
                    ...account,
                    name: updatedUser.name,
                    display_name: updatedUser.displayName,
                    picture: updatedUser.picture,
                    public_profile: updatedUser.publicProfile,
                    show_online_status: updatedUser.showOnlineStatus,
                  }
                : account,
            ),
          };
        }),

      updateRelayStatus: async () => {
        if (get().isFetchingRelayStatus) {
          return;
        }

        set({ isFetchingRelayStatus: true });

        try {
          const status = await getRelayStatus();
          set({
            relayStatus: status,
            relayStatusError: null,
            relayStatusBackoffMs: DEFAULT_RELAY_STATUS_INTERVAL,
            lastRelayStatusFetchedAt: Date.now(),
            isFetchingRelayStatus: false,
          });
        } catch (error) {
          errorHandler.log('Failed to get relay status', error, {
            context: 'AuthStore.updateRelayStatus',
          });
          const message = error instanceof Error ? error.message : 'Failed to get relay status';
          set({
            relayStatusError: message,
            relayStatusBackoffMs: nextRelayStatusBackoff(get().relayStatusBackoffMs),
            lastRelayStatusFetchedAt: Date.now(),
            isFetchingRelayStatus: false,
          });
        }
      },

      setRelayStatus: (status: RelayInfo[]) => {
        set({
          relayStatus: status,
          relayStatusError: null,
          relayStatusBackoffMs: DEFAULT_RELAY_STATUS_INTERVAL,
          lastRelayStatusFetchedAt: Date.now(),
          isFetchingRelayStatus: false,
        });
      },

      initialize: async () => {
        errorHandler.info('Auth store initialization started...', 'AuthStore.initialize');
        try {
          // セキュアストレージから現在のアカウントを取得
          const currentAccount = await SecureStorageApi.getCurrentAccount();
          errorHandler.info(
            `Current account from secure storage: ${currentAccount?.npub ?? 'unknown'}`,
            'AuthStore.initialize',
          );

          if (currentAccount) {
            errorHandler.info(
              `Auto-login with account: ${currentAccount.npub}`,
              'AuthStore.initialize',
            );
            // 自動ログイン
            const publicProfile =
              typeof currentAccount.metadata?.public_profile === 'boolean'
                ? currentAccount.metadata.public_profile
                : true;
            const showOnlineStatus =
              typeof currentAccount.metadata?.show_online_status === 'boolean'
                ? currentAccount.metadata.show_online_status
                : false;
            const user: User = {
              id: currentAccount.pubkey,
              pubkey: currentAccount.pubkey,
              npub: currentAccount.npub,
              name: currentAccount.metadata.name,
              displayName: currentAccount.metadata.display_name,
              about: '',
              picture: currentAccount.metadata.picture || '',
              nip05: '',
              avatar: null,
              publicProfile,
              showOnlineStatus,
            };

            set({
              isAuthenticated: true,
              currentUser: user,
              privateKey: currentAccount.nsec,
            });
            updateAuthDebug();

            // Nostrクライアントを初期化
            await initializeNostr();
            // リレー状態を更新
            await useAuthStore.getState().updateRelayStatus();
            await bootstrapTopics();
            errorHandler.info('Auto-login completed successfully', 'AuthStore.initialize');

            await fetchAndApplyAvatar(currentAccount.npub);
          } else {
            errorHandler.info('No current account found in secure storage', 'AuthStore.initialize');
            // アカウントが見つからない場合は初期状態
            set({
              isAuthenticated: false,
              currentUser: null,
              privateKey: null,
              relayStatus: [],
              relayStatusError: null,
              relayStatusBackoffMs: DEFAULT_RELAY_STATUS_INTERVAL,
              lastRelayStatusFetchedAt: null,
              isFetchingRelayStatus: false,
            });
            updateAuthDebug();
          }

          // アカウントリストを読み込み
          await useAuthStore.getState().loadAccounts();
          errorHandler.info('Auth store initialization completed', 'AuthStore.initialize');
        } catch (error) {
          errorHandler.log('Failed to initialize auth store', error, {
            context: 'AuthStore.initialize',
          });
          // エラー時は初期状態にリセット
          set({
            isAuthenticated: false,
            currentUser: null,
            privateKey: null,
            relayStatus: [],
            accounts: [],
            relayStatusError: null,
            relayStatusBackoffMs: DEFAULT_RELAY_STATUS_INTERVAL,
            lastRelayStatusFetchedAt: null,
            isFetchingRelayStatus: false,
          });
          updateAuthDebug();
        }
      },

      switchAccount: async (npub: string) => {
        const accountMetadata =
          get().accounts.find((account) => account.npub === npub) ||
          listFallbackAccountMetadata().find((account) => account.npub === npub);
        const metadataOverride = toUserOverride(accountMetadata);
        const fallbackNsec = getFallbackNsec(npub);
        errorHandler.info('Attempting account switch', 'AuthStore.switchAccount', {
          npub,
          hasFallback: Boolean(fallbackNsec),
          displayName: metadataOverride?.displayName,
        });

        const tryFallbackLogin = async () => {
          if (!fallbackNsec) {
            return false;
          }
          if (accountMetadata) {
            const fallbackUser: User = {
              id: accountMetadata.pubkey,
              pubkey: accountMetadata.pubkey,
              npub: accountMetadata.npub,
              name: accountMetadata.name,
              displayName: accountMetadata.display_name,
              about: '',
              picture: accountMetadata.picture ?? '',
              nip05: '',
              avatar: null,
              publicProfile:
                typeof accountMetadata.public_profile === 'boolean'
                  ? accountMetadata.public_profile
                  : true,
              showOnlineStatus:
                typeof accountMetadata.show_online_status === 'boolean'
                  ? accountMetadata.show_online_status
                  : false,
            };
            set({
              isAuthenticated: true,
              currentUser: fallbackUser,
              privateKey: fallbackNsec,
            });
            updateAuthDebug();
            hydratePrivacyFromUser(fallbackUser);
            try {
              await initializeNostr();
              await useAuthStore.getState().updateRelayStatus();
              await bootstrapTopics();
              await fetchAndApplyAvatar(accountMetadata.npub);
            } catch (fallbackError) {
              errorHandler.log('Fallback account switch initialization failed', fallbackError, {
                context: 'AuthStore.switchAccount.fallbackInitialize',
              });
            }
            errorHandler.info('Switched account via fallback metadata', 'AuthStore.switchAccount', {
              npub,
            });
            return true;
          }
          await get().loginWithNsec(fallbackNsec, false, metadataOverride);
          errorHandler.info('Switched account via fallback nsec', 'AuthStore.switchAccount', {
            npub,
          });
          return true;
        };

        if (await tryFallbackLogin()) {
          return;
        }

        try {
          const response = await SecureStorageApi.secureLogin(npub);

          const accounts = await SecureStorageApi.listAccounts();
          const account = accounts.find((a) => a.npub === npub);

          if (!account) {
            throw new Error('Account not found in secure storage');
          }

          const user: User = {
            id: response.public_key,
            pubkey: response.public_key,
            npub: response.npub,
            name: account.name,
            displayName: account.display_name,
            about: '',
            picture: account.picture || '',
            nip05: '',
            avatar: null,
            publicProfile:
              typeof account.public_profile === 'boolean' ? account.public_profile : true,
            showOnlineStatus:
              typeof account.show_online_status === 'boolean' ? account.show_online_status : false,
          };

          const mergedUser = applyUserMetadataOverride(user, metadataOverride);

          set({
            isAuthenticated: true,
            currentUser: mergedUser,
            privateKey: null,
          });
          updateAuthDebug();

          await initializeNostr();
          await useAuthStore.getState().updateRelayStatus();
          await bootstrapTopics();

          await fetchAndApplyAvatar(response.npub);

          errorHandler.info('Switched account via secure storage', 'AuthStore.switchAccount', {
            npub,
          });
          if (useAuthStore.getState().currentUser?.npub !== npub) {
            if (await tryFallbackLogin()) {
              return;
            }
          }
        } catch (error) {
          if (await tryFallbackLogin()) {
            return;
          }
          errorHandler.log('Failed to switch account', error, {
            context: 'AuthStore.switchAccount',
            showToast: true,
            toastTitle: '?????????????????',
          });
          throw error;
        }
      },

      removeAccount: async (npub: string) => {
        try {
          await SecureStorageApi.removeAccount(npub);
          removeFallbackAccount(npub);

          // 現在のアカウントが削除された場合はログアウト
          const currentUser = get().currentUser;
          if (currentUser?.npub === npub) {
            await get().logout();
          }

          // アカウントリストを更新
          await get().loadAccounts();
        } catch (error) {
          errorHandler.log('Failed to remove account', error, {
            context: 'AuthStore.removeAccount',
            showToast: true,
            toastTitle: 'アカウントの削除に失敗しました',
          });
          throw error;
        }
      },

      loadAccounts: async () => {
        try {
          const accounts = await SecureStorageApi.listAccounts();
          const fallback = listFallbackAccountMetadata();
          const merged = new Map<string, AccountMetadata>();
          for (const account of accounts) {
            merged.set(account.npub, account);
          }
          for (const account of fallback) {
            if (!merged.has(account.npub)) {
              merged.set(account.npub, account);
            }
          }
          const resolvedAccounts = Array.from(merged.values());
          set({ accounts: resolvedAccounts });
          updateAuthDebug();
        } catch (error) {
          errorHandler.log('Failed to load accounts', error, {
            context: 'AuthStore.loadAccounts',
          });
          set({ accounts: listFallbackAccountMetadata() });
          updateAuthDebug();
        }
      },

      get isLoggedIn() {
        return get().isAuthenticated;
      },
    };
  }, createAuthPersistConfig<AuthStore>()),
);
