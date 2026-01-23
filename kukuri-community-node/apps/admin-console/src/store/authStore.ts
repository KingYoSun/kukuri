import { create } from 'zustand';

import { api } from '../lib/api';
import { errorToMessage } from '../lib/errorHandler';
import type { AdminUser } from '../lib/types';

type AuthStatus = 'unknown' | 'checking' | 'authenticated' | 'unauthenticated';

type AuthState = {
  user: AdminUser | null;
  status: AuthStatus;
  error?: string;
  bootstrap: () => Promise<void>;
  login: (username: string, password: string) => Promise<void>;
  logout: () => Promise<void>;
};

const isUnauthorized = (err: unknown) => {
  const status = (err as { status?: number }).status;
  return status === 401 || status === 403;
};

export const useAuthStore = create<AuthState>((set, get) => ({
  user: null,
  status: 'unknown',
  error: undefined,
  bootstrap: async () => {
    if (get().status !== 'unknown') {
      return;
    }
    set({ status: 'checking', error: undefined });
    try {
      const user = await api.me();
      set({ user, status: 'authenticated', error: undefined });
    } catch (err) {
      if (isUnauthorized(err)) {
        set({ user: null, status: 'unauthenticated', error: undefined });
        return;
      }
      set({ user: null, status: 'unauthenticated', error: errorToMessage(err) });
    }
  },
  login: async (username: string, password: string) => {
    set({ status: 'checking', error: undefined });
    try {
      const user = await api.login(username, password);
      set({ user, status: 'authenticated', error: undefined });
    } catch (err) {
      set({ user: null, status: 'unauthenticated', error: errorToMessage(err) });
      throw err;
    }
  },
  logout: async () => {
    try {
      await api.logout();
    } finally {
      set({ user: null, status: 'unauthenticated', error: undefined });
    }
  }
}));
