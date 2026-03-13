import type { User } from '@/stores/types';

export const authorProfileCache = new Map<string, User>();
export const authorProfileMissedAt = new Map<string, number>();
export const authorProfileInFlight = new Map<string, Promise<User | null>>();

export function clearResolvedAuthorProfileCacheState(): void {
  authorProfileCache.clear();
  authorProfileMissedAt.clear();
  authorProfileInFlight.clear();
}
