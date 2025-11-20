import { SecureStorageApi } from '@/lib/api/secureStorage';
import { errorHandler } from '@/lib/errorHandler';
import { persistKeys } from '@/stores/config/persist';
import { useAuthStore } from '@/stores/authStore';

type AuthSnapshot = Pick<ReturnType<typeof useAuthStore.getState>, 'currentUser' | 'accounts'>;

interface E2EBridge {
  resetAppState: () => Promise<void>;
  getAuthSnapshot: () => AuthSnapshot;
}

declare global {
  interface Window {
    __KUKURI_E2E__?: E2EBridge;
  }
}

const PERSISTED_KEYS: string[] = [
  persistKeys.auth,
  persistKeys.drafts,
  persistKeys.offline,
  persistKeys.p2p,
  persistKeys.topic,
  persistKeys.privacy,
  persistKeys.keyManagement,
];

async function purgeSecureAccounts() {
  try {
    const accounts = await SecureStorageApi.listAccounts();
    for (const account of accounts) {
      try {
        await SecureStorageApi.removeAccount(account.npub);
      } catch (error) {
        errorHandler.log('E2EBridge.removeAccountFailed', error, {
          context: 'registerE2EBridge.purgeSecureAccounts',
          metadata: { npub: account.npub },
        });
      }
    }
  } catch (error) {
    errorHandler.log('E2EBridge.listAccountsFailed', error, {
      context: 'registerE2EBridge.purgeSecureAccounts',
    });
  }
}

function clearPersistedState() {
  if (typeof window === 'undefined') {
    return;
  }
  for (const key of PERSISTED_KEYS) {
    window.localStorage?.removeItem(key);
  }
}

async function resetAuthStore() {
  try {
    await useAuthStore.getState().logout();
  } catch (error) {
    errorHandler.log('E2EBridge.logoutFailed', error, {
      context: 'registerE2EBridge.resetAuthStore',
    });
  }
  useAuthStore.setState({
    isAuthenticated: false,
    currentUser: null,
    privateKey: null,
    accounts: [],
  });
}

export function registerE2EBridge(): void {
  if (typeof window === 'undefined') {
    return;
  }
  if (window.__KUKURI_E2E__) {
    return;
  }

  window.__KUKURI_E2E__ = {
    resetAppState: async () => {
      await purgeSecureAccounts();
      clearPersistedState();
      await resetAuthStore();
    },
    getAuthSnapshot: () => {
      const state = useAuthStore.getState();
      return {
        currentUser: state.currentUser,
        accounts: state.accounts,
      };
    },
  };
}
