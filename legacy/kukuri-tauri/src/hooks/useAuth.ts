import { useQueryClient } from '@tanstack/react-query';
import { useAuthStore } from '@/stores';

// 認証状態と操作を提供する汎用フック
export const useAuth = () => {
  const store = useAuthStore();
  return {
    isAuthenticated: store.isAuthenticated,
    currentUser: store.currentUser,
    relayStatus: store.relayStatus,
    isLoggedIn: store.isLoggedIn,
    login: store.login,
    loginWithNsec: store.loginWithNsec,
    generateNewKeypair: store.generateNewKeypair,
    logout: store.logout,
    updateUser: store.updateUser,
    updateRelayStatus: store.updateRelayStatus,
    initialize: store.initialize,
  };
};

// ログアウト時のクエリキャッシュクリア
export const useLogout = () => {
  const queryClient = useQueryClient();
  const { logout } = useAuthStore();

  return async () => {
    await logout();
    queryClient.clear();
  };
};
