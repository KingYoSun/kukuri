import type { UserProfile } from '@/lib/api/tauri';
import type { User } from '@/stores/types';
import { applyKnownUserMetadata } from './userMetadata';

export function mapUserProfileToUser(profile: UserProfile): User {
  const base: User = {
    id: profile.pubkey,
    pubkey: profile.pubkey,
    npub: profile.npub,
    name: profile.name ?? '',
    displayName: profile.display_name ?? profile.name ?? '',
    about: profile.about ?? '',
    picture: profile.picture ?? '',
    nip05: profile.nip05 ?? '',
    avatar: null,
  };

  return applyKnownUserMetadata(base);
}
