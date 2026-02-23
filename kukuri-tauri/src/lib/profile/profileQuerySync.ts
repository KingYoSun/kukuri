import type { QueryClient } from '@tanstack/react-query';
import type { User } from '@/stores/types';

function toUserProfileDto(user: User) {
  return {
    npub: user.npub,
    pubkey: user.pubkey,
    name: user.name,
    display_name: user.displayName,
    about: user.about,
    picture: user.picture,
    banner: null,
    website: null,
    nip05: user.nip05,
    is_profile_public: user.publicProfile,
    show_online_status: user.showOnlineStatus,
  };
}

export function syncProfileQueryCaches(queryClient: QueryClient, user: User): void {
  const dto = toUserProfileDto(user);

  queryClient.setQueryData(['userProfile', user.npub], dto);
  queryClient.setQueryData(['userProfile', user.pubkey], dto);

  void Promise.allSettled([
    queryClient.invalidateQueries({ queryKey: ['userProfile'] }),
    queryClient.invalidateQueries({ queryKey: ['user-search'] }),
    queryClient.invalidateQueries({ queryKey: ['profile'] }),
    queryClient.invalidateQueries({ queryKey: ['userPosts', user.pubkey] }),
    queryClient.invalidateQueries({ queryKey: ['followingFeed'] }),
    queryClient.invalidateQueries({ queryKey: ['trending', 'posts'] }),
  ]);
}
