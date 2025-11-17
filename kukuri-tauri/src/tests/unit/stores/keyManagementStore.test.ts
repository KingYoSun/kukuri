import { describe, it, expect, beforeEach, vi } from 'vitest';

import { useKeyManagementStore } from '@/stores/keyManagementStore';

describe('useKeyManagementStore', () => {
  beforeEach(() => {
    useKeyManagementStore.setState((state) => ({
      ...state,
      history: [],
      lastExportedAt: null,
      lastImportedAt: null,
    }));
  });

  it('records export actions and updates the timestamp', () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date('2025-11-16T12:00:00Z'));
    useKeyManagementStore.getState().recordAction({
      action: 'export',
      status: 'success',
      metadata: { stage: 'fetch' },
    });

    const state = useKeyManagementStore.getState();
    expect(state.history).toHaveLength(1);
    expect(state.history[0].action).toBe('export');
    expect(state.lastExportedAt).toBe(state.history[0].timestamp);
    expect(state.lastImportedAt).toBeNull();
    vi.useRealTimers();
  });

  it('records import actions and keeps only the latest entries', () => {
    for (let i = 0; i < 25; i += 1) {
      useKeyManagementStore.getState().recordAction({
        action: 'import',
        status: 'success',
      });
    }
    const state = useKeyManagementStore.getState();
    expect(state.history).toHaveLength(20);
    expect(state.history[0].action).toBe('import');
    expect(state.lastImportedAt).toBe(state.history[0].timestamp);
  });

  it('clears history values', () => {
    useKeyManagementStore.getState().recordAction({
      action: 'export',
      status: 'success',
    });
    useKeyManagementStore.getState().recordAction({
      action: 'import',
      status: 'success',
    });

    useKeyManagementStore.getState().clearHistory();
    const state = useKeyManagementStore.getState();
    expect(state.history).toHaveLength(0);
    expect(state.lastExportedAt).toBeNull();
    expect(state.lastImportedAt).toBeNull();
  });
});
