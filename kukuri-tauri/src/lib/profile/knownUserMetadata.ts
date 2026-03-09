import type { UserProfile } from '@/lib/api/tauri';
import type { User } from '@/stores/types';

const knownUserMetadata = new Map<string, User>();

const normalizeKey = (value: string | null | undefined): string | null => {
  const trimmed = value?.trim().toLowerCase();
  return trimmed && trimmed.length > 0 ? trimmed : null;
};

const preferString = (primary: string | null | undefined, fallback = ''): string => {
  const trimmedPrimary = primary?.trim();
  if (trimmedPrimary) {
    return trimmedPrimary;
  }
  return fallback.trim();
};

const normalizeUser = (user: User): User => ({
  ...user,
  name: user.name ?? '',
  displayName: user.displayName ?? user.name ?? '',
  picture: user.picture ?? '',
  about: user.about ?? '',
  nip05: user.nip05 ?? '',
  avatar: user.avatar ?? null,
  publicProfile: typeof user.publicProfile === 'boolean' ? user.publicProfile : true,
  showOnlineStatus: typeof user.showOnlineStatus === 'boolean' ? user.showOnlineStatus : false,
});

const mergeUsers = (existing: User | null, incoming: User): User => {
  const normalizedIncoming = normalizeUser(incoming);
  if (!existing) {
    return normalizedIncoming;
  }

  return normalizeUser({
    ...existing,
    ...normalizedIncoming,
    id: preferString(normalizedIncoming.id, existing.id),
    pubkey: preferString(normalizedIncoming.pubkey, existing.pubkey),
    npub: preferString(normalizedIncoming.npub, existing.npub),
    name: preferString(normalizedIncoming.name, existing.name),
    displayName: preferString(
      normalizedIncoming.displayName,
      normalizedIncoming.name || existing.displayName,
    ),
    picture: preferString(normalizedIncoming.picture, existing.picture),
    about: preferString(normalizedIncoming.about, existing.about),
    nip05: preferString(normalizedIncoming.nip05, existing.nip05),
    avatar: normalizedIncoming.avatar ?? existing.avatar ?? null,
    publicProfile:
      typeof normalizedIncoming.publicProfile === 'boolean'
        ? normalizedIncoming.publicProfile
        : existing.publicProfile,
    showOnlineStatus:
      typeof normalizedIncoming.showOnlineStatus === 'boolean'
        ? normalizedIncoming.showOnlineStatus
        : existing.showOnlineStatus,
  });
};

const storeKnownUser = (user: User): User => {
  const keys = [normalizeKey(user.npub), normalizeKey(user.pubkey), normalizeKey(user.id)].filter(
    (value): value is string => Boolean(value),
  );
  const existing =
    keys
      .map((key) => knownUserMetadata.get(key) ?? null)
      .find((value): value is User => value !== null) ?? null;
  const merged = mergeUsers(existing, user);

  keys.forEach((key) => {
    knownUserMetadata.set(key, merged);
  });

  return merged;
};

export function rememberKnownUserMetadata(user: User): User {
  return storeKnownUser(user);
}

export function rememberKnownUserProfile(profile: UserProfile): User {
  return storeKnownUser({
    id: profile.pubkey,
    pubkey: profile.pubkey,
    npub: profile.npub,
    name: profile.name ?? '',
    displayName: profile.display_name ?? profile.name ?? '',
    about: profile.about ?? '',
    picture: profile.picture ?? '',
    nip05: profile.nip05 ?? '',
    publicProfile: profile.is_profile_public ?? true,
    showOnlineStatus: profile.show_online_status ?? false,
    avatar: null,
  });
}

export function resolveKnownUserMetadata(user: Pick<User, 'npub' | 'pubkey' | 'id'>): User | null {
  const keys = [normalizeKey(user.npub), normalizeKey(user.pubkey), normalizeKey(user.id)].filter(
    (value): value is string => Boolean(value),
  );

  for (const key of keys) {
    const remembered = knownUserMetadata.get(key);
    if (remembered) {
      return remembered;
    }
  }

  return null;
}

export function clearKnownUserMetadata(): void {
  knownUserMetadata.clear();
}
