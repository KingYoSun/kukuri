import { describe, expect, it, vi } from 'vitest';

import {
  COMMUNITY_INDEX_NODE_PREFERENCE_STORAGE_KEY,
  readCommunityIndexNodePreference,
  startCommunityIndexNodePreferencePersistence,
  writeCommunityIndexNodePreference,
  type CommunityIndexNodePreferenceStorage,
} from '@/shell/communityIndexNodePreference';
import { createDesktopShellStore } from '@/shell/store';

function memoryStorage(initial: string | null = null): CommunityIndexNodePreferenceStorage & {
  getItem: ReturnType<typeof vi.fn>;
  setItem: ReturnType<typeof vi.fn>;
} {
  let value = initial;
  return {
    getItem: vi.fn(() => value),
    setItem: vi.fn((_key: string, next: string) => {
      value = next;
    }),
  };
}

describe('community index node preference persistence', () => {
  it('round-trips auto and manual preferences', () => {
    const storage = memoryStorage();
    expect(writeCommunityIndexNodePreference(storage, { mode: 'manual', baseUrl: 'https://b' })).toBe(true);
    expect(storage.setItem).toHaveBeenCalledWith(
      COMMUNITY_INDEX_NODE_PREFERENCE_STORAGE_KEY,
      expect.any(String)
    );
    expect(readCommunityIndexNodePreference(storage)).toEqual({
      mode: 'manual',
      baseUrl: 'https://b',
    });
    expect(writeCommunityIndexNodePreference(storage, { mode: 'auto' })).toBe(true);
    expect(readCommunityIndexNodePreference(storage)).toEqual({ mode: 'auto' });
  });

  it('falls back to auto for malformed payloads and storage failures', () => {
    expect(readCommunityIndexNodePreference(memoryStorage('{'))).toEqual({ mode: 'auto' });
    expect(readCommunityIndexNodePreference(memoryStorage(JSON.stringify({ version: 2 })))).toEqual({ mode: 'auto' });
    expect(readCommunityIndexNodePreference(memoryStorage(JSON.stringify({
      version: 1,
      preference: { mode: 'manual', baseUrl: '' },
    })))).toEqual({ mode: 'auto' });
    expect(readCommunityIndexNodePreference({
      getItem: () => { throw new Error('denied'); },
      setItem: vi.fn(),
    })).toEqual({ mode: 'auto' });
  });

  it('persists only preference changes and unsubscribes cleanly', () => {
    const storage = memoryStorage();
    const store = createDesktopShellStore();
    const unsubscribe = startCommunityIndexNodePreferencePersistence(store, storage);
    store.getState().setField('topicInput', 'unrelated');
    expect(storage.setItem).not.toHaveBeenCalled();
    store.getState().setField('communityIndexNodePreference', {
      mode: 'manual',
      baseUrl: 'https://b',
    });
    expect(storage.setItem).toHaveBeenCalledTimes(1);
    unsubscribe();
    store.getState().setField('communityIndexNodePreference', { mode: 'auto' });
    expect(storage.setItem).toHaveBeenCalledTimes(1);
  });
});
