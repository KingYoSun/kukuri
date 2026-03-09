import { TauriApi } from '@/lib/api/tauri';
import { subscribeToUser } from '@/lib/api/nostr';
import { errorHandler } from '@/lib/errorHandler';
import type { User } from '@/stores/types';
import { isHexFormat, isNpubFormat, npubToPubkey, pubkeyToNpub } from '@/lib/utils/nostr';
import { rememberKnownUserMetadata } from './knownUserMetadata';
import { mapUserProfileToUser } from './profileMapper';
import {
  authorProfileCache,
  authorProfileInFlight,
  authorProfileMissedAt,
  clearResolvedAuthorProfileCacheState,
} from './authorProfileResolverCache';

const AUTHOR_PROFILE_MISS_TTL_MS = 60_000;
const AUTHOR_PROFILE_POLL_INTERVAL_MS = 250;
const AUTHOR_PROFILE_POLL_TIMEOUT_MS = 5_000;

const normalizeKey = (value: string): string => value.trim().toLowerCase();

const wait = async (ms: number): Promise<void> => {
  await new Promise((resolve) => globalThis.setTimeout(resolve, ms));
};

const buildCacheKeys = (values: Array<string | null | undefined>): string[] => {
  const seen = new Set<string>();
  const keys: string[] = [];

  values.forEach((value) => {
    const trimmed = value?.trim();
    if (!trimmed) {
      return;
    }

    const normalized = normalizeKey(trimmed);
    if (seen.has(normalized)) {
      return;
    }

    seen.add(normalized);
    keys.push(normalized);
  });

  return keys;
};

const rememberResolvedProfile = (user: User, cacheKeys: string[]): User => {
  const remembered = rememberKnownUserMetadata(user);
  cacheKeys.forEach((key) => {
    authorProfileCache.set(key, remembered);
    authorProfileMissedAt.delete(key);
  });
  return remembered;
};

const markProfileMiss = (cacheKeys: string[]): void => {
  const now = Date.now();
  cacheKeys.forEach((key) => {
    authorProfileMissedAt.set(key, now);
  });
};

const recentlyMissed = (cacheKeys: string[]): boolean =>
  cacheKeys.some((key) => {
    const missedAt = authorProfileMissedAt.get(key);
    return Boolean(missedAt && Date.now() - missedAt < AUTHOR_PROFILE_MISS_TTL_MS);
  });

const loadPersistedProfile = async (
  authorPubkey: string | null,
  authorNpub: string | null,
): Promise<User | null> => {
  const byPubkey =
    authorPubkey && isHexFormat(authorPubkey)
      ? await TauriApi.getUserProfileByPubkey(authorPubkey)
      : null;
  const byNpub =
    !byPubkey && authorNpub && isNpubFormat(authorNpub)
      ? await TauriApi.getUserProfile(authorNpub)
      : null;
  const profile = byPubkey ?? byNpub;
  return profile ? mapUserProfileToUser(profile) : null;
};

const waitForPersistedProfile = async (
  authorPubkey: string | null,
  authorNpub: string | null,
): Promise<User | null> => {
  const deadline = Date.now() + AUTHOR_PROFILE_POLL_TIMEOUT_MS;

  while (Date.now() <= deadline) {
    const profile = await loadPersistedProfile(authorPubkey, authorNpub);
    if (profile) {
      return profile;
    }
    await wait(AUTHOR_PROFILE_POLL_INTERVAL_MS);
  }

  return null;
};

const resolveAuthorIdentifiers = async (
  author: string,
): Promise<{ authorPubkey: string | null; authorNpub: string | null; cacheKeys: string[] }> => {
  const trimmed = author.trim();
  if (!trimmed) {
    return { authorPubkey: null, authorNpub: null, cacheKeys: [] };
  }

  if (isHexFormat(trimmed)) {
    const authorNpub = await pubkeyToNpub(trimmed);
    return {
      authorPubkey: trimmed,
      authorNpub,
      cacheKeys: buildCacheKeys([trimmed, authorNpub]),
    };
  }

  if (isNpubFormat(trimmed)) {
    const authorPubkey = await npubToPubkey(trimmed);
    return {
      authorPubkey: isHexFormat(authorPubkey) ? authorPubkey : null,
      authorNpub: trimmed,
      cacheKeys: buildCacheKeys([trimmed, authorPubkey]),
    };
  }

  return {
    authorPubkey: null,
    authorNpub: null,
    cacheKeys: buildCacheKeys([trimmed]),
  };
};

export async function resolveAuthorProfileWithRelayFallback(author: string): Promise<User | null> {
  const normalizedAuthor = author.trim();
  if (!normalizedAuthor) {
    return null;
  }

  const directCacheKey = normalizeKey(normalizedAuthor);
  const cached = authorProfileCache.get(directCacheKey);
  if (cached) {
    return cached;
  }

  const inFlight = authorProfileInFlight.get(directCacheKey);
  if (inFlight) {
    return await inFlight;
  }

  const loader = (async (): Promise<User | null> => {
    const { authorPubkey, authorNpub, cacheKeys } =
      await resolveAuthorIdentifiers(normalizedAuthor);
    const effectiveCacheKeys = buildCacheKeys([normalizedAuthor, ...cacheKeys]);

    for (const key of effectiveCacheKeys) {
      const resolved = authorProfileCache.get(key);
      if (resolved) {
        return resolved;
      }
    }

    if (recentlyMissed(effectiveCacheKeys)) {
      return null;
    }

    const persisted = await loadPersistedProfile(authorPubkey, authorNpub);
    if (persisted) {
      return rememberResolvedProfile(persisted, effectiveCacheKeys);
    }

    if (authorPubkey && isHexFormat(authorPubkey)) {
      try {
        await subscribeToUser(authorPubkey);
      } catch (error) {
        errorHandler.log('Failed to subscribe for missing author profile', error, {
          context: 'authorProfileResolver.subscribeToUser',
          showToast: false,
          metadata: { author: normalizedAuthor, authorPubkey },
        });
        markProfileMiss(effectiveCacheKeys);
        return null;
      }

      const resolvedAfterSubscribe = await waitForPersistedProfile(authorPubkey, authorNpub);
      if (resolvedAfterSubscribe) {
        return rememberResolvedProfile(resolvedAfterSubscribe, effectiveCacheKeys);
      }
    }

    markProfileMiss(effectiveCacheKeys);
    return null;
  })();

  authorProfileInFlight.set(directCacheKey, loader);

  try {
    return await loader;
  } finally {
    authorProfileInFlight.delete(directCacheKey);
  }
}

export function clearResolvedAuthorProfileCache(): void {
  clearResolvedAuthorProfileCacheState();
}
