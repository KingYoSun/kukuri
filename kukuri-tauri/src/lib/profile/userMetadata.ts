import type { User } from '@/stores/types';
import type { AccountMetadata } from '@/lib/api/secureStorage';
import { useAuthStore } from '@/stores/authStore';

function isSameUser(target: User, reference: User | AccountMetadata): boolean {
  const targetNpub = target.npub?.toLowerCase();
  const targetPubkey = target.pubkey?.toLowerCase();

  const refNpub = 'npub' in reference ? reference.npub?.toLowerCase() : undefined;
  const refPubkey = 'pubkey' in reference ? reference.pubkey?.toLowerCase() : undefined;

  if (targetNpub && refNpub && targetNpub === refNpub) {
    return true;
  }

  if (targetPubkey && refPubkey && targetPubkey === refPubkey) {
    return true;
  }

  return false;
}

export function applyKnownUserMetadata(user: User): User {
  const authState = useAuthStore.getState();
  const base: User = {
    ...user,
    avatar: user.avatar ?? null,
    picture: user.picture ?? '',
    name: user.name ?? '',
    displayName: user.displayName ?? user.name ?? '',
    about: user.about ?? '',
    nip05: user.nip05 ?? '',
  };

  const currentUser = authState.currentUser;
  if (currentUser && isSameUser(base, currentUser)) {
    return {
      ...base,
      name: currentUser.name || base.name,
      displayName: currentUser.displayName || currentUser.name || base.displayName,
      about: currentUser.about ?? base.about,
      picture: currentUser.picture?.trim() ? currentUser.picture : base.picture,
      nip05: currentUser.nip05 || base.nip05,
      avatar: currentUser.avatar ?? base.avatar ?? null,
    };
  }

  const account = authState.accounts?.find((item) => isSameUser(base, item)) ?? null;

  if (account) {
    return {
      ...base,
      name: account.name || base.name,
      displayName: account.display_name || account.name || base.displayName,
      picture: account.picture?.trim() ? account.picture : base.picture,
    };
  }

  return base;
}
