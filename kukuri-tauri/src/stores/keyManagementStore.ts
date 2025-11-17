import { create } from 'zustand';
import { withPersist } from './utils/persistHelpers';
import { createKeyManagementPersistConfig } from './config/persist';

export type KeyManagementAction = 'export' | 'import';
export type KeyManagementStatus = 'success' | 'error' | 'cancelled';

export interface KeyManagementHistoryEntry {
  id: string;
  action: KeyManagementAction;
  status: KeyManagementStatus;
  timestamp: number;
  metadata?: Record<string, string>;
}

interface KeyManagementStore {
  history: KeyManagementHistoryEntry[];
  lastExportedAt: number | null;
  lastImportedAt: number | null;
  recordAction: (entry: {
    action: KeyManagementAction;
    status: KeyManagementStatus;
    metadata?: Record<string, string>;
  }) => void;
  clearHistory: () => void;
}

const MAX_HISTORY_ENTRIES = 20;

const createEntryId = (action: KeyManagementAction) => {
  if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
    return crypto.randomUUID();
  }
  return `${action}-${Date.now()}-${Math.random().toString(16).slice(2)}`;
};

export const useKeyManagementStore = create<KeyManagementStore>()(
  withPersist<KeyManagementStore>(
    (set) => ({
      history: [],
      lastExportedAt: null,
      lastImportedAt: null,
      recordAction: ({ action, status, metadata }) => {
        const entry: KeyManagementHistoryEntry = {
          id: createEntryId(action),
          action,
          status,
          timestamp: Date.now(),
          metadata,
        };
        set((state) => {
          const history = [entry, ...state.history].slice(0, MAX_HISTORY_ENTRIES);
          return {
            history,
            lastExportedAt:
              action === 'export' && status === 'success' ? entry.timestamp : state.lastExportedAt,
            lastImportedAt:
              action === 'import' && status === 'success' ? entry.timestamp : state.lastImportedAt,
          };
        });
      },
      clearHistory: () =>
        set({
          history: [],
          lastExportedAt: null,
          lastImportedAt: null,
        }),
    }),
    createKeyManagementPersistConfig<KeyManagementStore>(),
  ),
);
