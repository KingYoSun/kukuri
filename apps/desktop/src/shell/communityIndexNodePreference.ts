import type { CommunityIndexNodePreference } from '@/lib/api/communityIndex';
import type { DesktopShellStoreApi } from '@/shell/store';

export const COMMUNITY_INDEX_NODE_PREFERENCE_STORAGE_KEY =
  'kukuri:community-index-node-preference:v1';
const COMMUNITY_INDEX_NODE_PREFERENCE_STORAGE_VERSION = 1;

export type CommunityIndexNodePreferenceStorage = Pick<Storage, 'getItem' | 'setItem'>;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function serializePreference(preference: CommunityIndexNodePreference) {
  return JSON.stringify({
    version: COMMUNITY_INDEX_NODE_PREFERENCE_STORAGE_VERSION,
    preference,
  });
}

export function readCommunityIndexNodePreference(
  storage: CommunityIndexNodePreferenceStorage
): CommunityIndexNodePreference {
  try {
    const raw = storage.getItem(COMMUNITY_INDEX_NODE_PREFERENCE_STORAGE_KEY);
    if (!raw) return { mode: 'auto' };
    const parsed: unknown = JSON.parse(raw);
    if (
      !isRecord(parsed) ||
      parsed.version !== COMMUNITY_INDEX_NODE_PREFERENCE_STORAGE_VERSION ||
      !isRecord(parsed.preference)
    ) return { mode: 'auto' };
    if (parsed.preference.mode === 'auto') return { mode: 'auto' };
    if (
      parsed.preference.mode === 'manual' &&
      typeof parsed.preference.baseUrl === 'string' &&
      parsed.preference.baseUrl.trim()
    ) {
      return { mode: 'manual', baseUrl: parsed.preference.baseUrl.trim() };
    }
    return { mode: 'auto' };
  } catch {
    return { mode: 'auto' };
  }
}

export function writeCommunityIndexNodePreference(
  storage: CommunityIndexNodePreferenceStorage,
  preference: CommunityIndexNodePreference
) {
  try {
    storage.setItem(
      COMMUNITY_INDEX_NODE_PREFERENCE_STORAGE_KEY,
      serializePreference(preference)
    );
    return true;
  } catch {
    return false;
  }
}

export function startCommunityIndexNodePreferencePersistence(
  store: DesktopShellStoreApi,
  storage: CommunityIndexNodePreferenceStorage
) {
  let previous = serializePreference(store.getState().communityIndexNodePreference);
  return store.subscribe((state) => {
    const next = serializePreference(state.communityIndexNodePreference);
    if (next === previous) return;
    previous = next;
    writeCommunityIndexNodePreference(storage, state.communityIndexNodePreference);
  });
}
