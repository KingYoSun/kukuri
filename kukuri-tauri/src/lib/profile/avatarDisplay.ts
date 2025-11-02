import defaultAvatar from '@/assets/profile/default_avatar.png';
import type { UserAvatarMetadata } from '@/stores/types';

interface AvatarOwner {
  picture?: string | null;
  avatar?: UserAvatarMetadata | null;
}

export function resolveAvatarSrc(picture?: string | null): string {
  const trimmed = picture?.trim();
  return trimmed && trimmed.length > 0 ? trimmed : defaultAvatar;
}

export function resolveUserAvatarSrc(user?: AvatarOwner | null): string {
  if (!user) {
    return defaultAvatar;
  }

  const picture = user.picture?.trim();
  if (picture && picture.length > 0) {
    return picture;
  }

  const nostrUri = user.avatar?.nostrUri?.trim();
  if (nostrUri && nostrUri.length > 0) {
    return nostrUri;
  }

  return defaultAvatar;
}
