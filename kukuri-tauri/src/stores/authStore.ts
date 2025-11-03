import { create } from 'zustand';

import type { AuthState, User } from './types';
import { TauriApi } from '@/lib/api/tauri';
import { initializeNostr, disconnectNostr, getRelayStatus, type RelayInfo } from '@/lib/api/nostr';
import { SecureStorageApi, type AccountMetadata } from '@/lib/api/secureStorage';
import { errorHandler } from '@/lib/errorHandler';
import { useTopicStore } from './topicStore';
import { withPersist } from './utils/persistHelpers';
import { createAuthPersistConfig } from './config/persist';
import { buildAvatarDataUrl, buildUserAvatarMetadataFromFetch } from '@/lib/profile/avatar';

const DEFAULT_RELAY_STATUS_INTERVAL = 30_000;
const RELAY_STATUS_BACKOFF_SEQUENCE = [120_000, 300_000, 600_000];

const nextRelayStatusBackoff = (current: number) => {
  for (const value of RELAY_STATUS_BACKOFF_SEQUENCE) {
    if (current < value) {
      return value;
    }
  }
  return RELAY_STATUS_BACKOFF_SEQUENCE[RELAY_STATUS_BACKOFF_SEQUENCE.length - 1];
};

interface AuthStore extends AuthState {
  relayStatus: RelayInfo[];
  relayStatusError: string | null;
  relayStatusBackoffMs: number;
  lastRelayStatusFetchedAt: number | null;
  isFetchingRelayStatus: boolean;
  accounts: AccountMetadata[];
  login: (privateKey: string, user: User) => Promise<void>;
  loginWithNsec: (nsec: string, saveToSecureStorage?: boolean) => Promise<void>;
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
        try {
          await initializeNostr();
          await fetchAndApplyAvatar(user.npub);
        } catch (error) {
          errorHandler.log('Failed to initialize Nostr', error, {
            context: 'AuthStore.login',
          });
        }
      },

      loginWithNsec: async (nsec: string, saveToSecureStorage = false) => {
        try {
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
            avatar: null,
          };

          // セキュアストレージに保存
          if (saveToSecureStorage) {
            await SecureStorageApi.addAccount({
              nsec,
              name: user.name,
              display_name: user.displayName,
              picture: user.picture,
            });
          }

          set({
            isAuthenticated: true,
            currentUser: user,
            privateKey: nsec,
          });

          // Nostrクライアントを初期化
          await initializeNostr();
          // リレー状態を更新
          await useAuthStore.getState().updateRelayStatus();
          // アカウントリストを更新
          await useAuthStore.getState().loadAccounts();

          // 初回ログイン時（アカウント追加時）は#publicトピックに参加
          if (saveToSecureStorage) {
            const topicStore = useTopicStore.getState();
            // トピック一覧を取得
            await topicStore.fetchTopics();
            // #publicトピックを探す
            const publicTopic = Array.from(topicStore.topics.values()).find(
              (t) => t.id === 'public',
            );
            if (publicTopic) {
              // #publicトピックに参加
              await topicStore.joinTopic('public');
              // #publicトピックをデフォルト表示に設定
              topicStore.setCurrentTopic(publicTopic);
            }
          }

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
            avatar: null,
          };

          // セキュアストレージに保存
          if (saveToSecureStorage) {
            errorHandler.info(
              'Saving new account to secure storage...',
              'AuthStore.generateNewKeypair',
            );
            await SecureStorageApi.addAccount({
              nsec: response.nsec,
              name: user.name,
              display_name: user.displayName,
              picture: user.picture,
            });
            errorHandler.info('Account saved successfully', 'AuthStore.generateNewKeypair');
          }

          set({
            isAuthenticated: true,
            currentUser: user,
            privateKey: response.nsec,
          });

          // Nostrクライアントを初期化
          await initializeNostr();
          // リレー状態を更新
          await useAuthStore.getState().updateRelayStatus();
          // アカウントリストを更新
          await useAuthStore.getState().loadAccounts();

          // 新規アカウント作成時は#publicトピックに参加
          if (saveToSecureStorage) {
            const topicStore = useTopicStore.getState();
            // トピック一覧を取得
            await topicStore.fetchTopics();
            // #publicトピックを探す
            const publicTopic = Array.from(topicStore.topics.values()).find(
              (t) => t.id === 'public',
            );
            if (publicTopic) {
              // #publicトピックに参加
              await topicStore.joinTopic('public');
              // #publicトピックをデフォルト表示に設定
              topicStore.setCurrentTopic(publicTopic);
            }
          }

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
      },

      updateUser: (userUpdate: Partial<User>) =>
        set((state) => ({
          currentUser: state.currentUser
            ? {
                ...state.currentUser,
                ...userUpdate,
              }
            : null,
        })),

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
          const message =
            error instanceof Error ? error.message : 'Failed to get relay status';
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
            };

            set({
              isAuthenticated: true,
              currentUser: user,
              privateKey: currentAccount.nsec,
            });

            // Nostrクライアントを初期化
            await initializeNostr();
            // リレー状態を更新
            await useAuthStore.getState().updateRelayStatus();
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
        }
      },

      switchAccount: async (npub: string) => {
        try {
          // セキュアストレージからログイン
          const response = await SecureStorageApi.secureLogin(npub);

          // アカウント情報を取得
          const accounts = await SecureStorageApi.listAccounts();
          const account = accounts.find((a) => a.npub === npub);

          if (!account) {
            throw new Error('Account not found');
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
          };

          set({
            isAuthenticated: true,
            currentUser: user,
            privateKey: null, // セキュアストレージから取得したものは保持しない
          });

          // Nostrクライアントを初期化
          await initializeNostr();
          // リレー状態を更新
          await useAuthStore.getState().updateRelayStatus();

          await fetchAndApplyAvatar(response.npub);
        } catch (error) {
          errorHandler.log('Failed to switch account', error, {
            context: 'AuthStore.switchAccount',
            showToast: true,
            toastTitle: 'アカウントの切り替えに失敗しました',
          });
          throw error;
        }
      },

      removeAccount: async (npub: string) => {
        try {
          await SecureStorageApi.removeAccount(npub);

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
          set({ accounts });
        } catch (error) {
          errorHandler.log('Failed to load accounts', error, {
            context: 'AuthStore.loadAccounts',
          });
          set({ accounts: [] });
        }
      },

      get isLoggedIn() {
        return get().isAuthenticated;
      },
    };
  }, createAuthPersistConfig<AuthStore>()),
);
