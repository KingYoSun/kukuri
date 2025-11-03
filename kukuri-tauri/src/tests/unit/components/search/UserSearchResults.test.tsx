import React from 'react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen, waitFor } from '@testing-library/react';
import { vi, type SpyInstance } from 'vitest';

import { UserSearchResults } from '@/components/search/UserSearchResults';
import { resolveUserAvatarSrc } from '@/lib/profile/avatarDisplay';
import { TauriApi } from '@/lib/api/tauri';
import { useAuthStore } from '@/stores/authStore';

vi.mock('@tanstack/react-router', () => ({
  Link: ({
    children,
    to,
    ...rest
  }: {
    children: React.ReactNode;
    to: string;
    params?: Record<string, unknown>;
  }) => (
    <a href={typeof to === 'string' ? to : '#'} {...rest}>
      {children}
    </a>
  ),
}));

const avatarImageSources: string[] = [];

vi.mock('@/components/ui/avatar', () => ({
  Avatar: ({
    children,
    ...props
  }: React.HTMLAttributes<HTMLDivElement> & { children?: React.ReactNode }) => (
    <div data-slot="avatar" {...props}>
      {children}
    </div>
  ),
  AvatarImage: ({
    src,
    ...props
  }: React.ImgHTMLAttributes<HTMLImageElement> & { src?: string | undefined }) => {
    const resolved = typeof src === 'string' ? src : '';
    avatarImageSources.push(resolved);
    return <img data-slot="avatar-image" src={resolved} {...props} />;
  },
  AvatarFallback: ({
    children,
    ...props
  }: React.HTMLAttributes<HTMLSpanElement> & { children?: React.ReactNode }) => (
    <span data-slot="avatar-fallback" {...props}>
      {children}
    </span>
  ),
}));

describe('UserSearchResults', () => {
  const originalAuthState = useAuthStore.getState();
  let searchUsersSpy: SpyInstance;
  let getFollowingSpy: SpyInstance;

  beforeEach(() => {
    avatarImageSources.length = 0;
    searchUsersSpy = vi
      .spyOn(TauriApi, 'searchUsers')
      .mockResolvedValue([
      {
        npub: 'npub1alice',
        pubkey: 'pubkey1alice',
        name: 'alice',
        display_name: 'Alice',
        about: 'Nostr開発者',
        picture: '',
        banner: null,
        website: null,
        nip05: 'alice@example.com',
      },
    ]);
    getFollowingSpy = vi.spyOn(TauriApi, 'getFollowing').mockResolvedValue([]);
    useAuthStore.setState({
      ...originalAuthState,
      isAuthenticated: false,
      currentUser: null,
    });
  });

  afterEach(() => {
    searchUsersSpy.mockRestore();
    getFollowingSpy.mockRestore();
    useAuthStore.setState(originalAuthState);
  });

  const renderWithQueryClient = (query: string) => {
    const queryClient = new QueryClient({
      defaultOptions: {
        queries: {
          retry: false,
          cacheTime: 0,
          staleTime: 0,
        },
      },
    });

    const utils = render(
      <QueryClientProvider client={queryClient}>
        <UserSearchResults query={query} />
      </QueryClientProvider>,
    );

    return {
      ...utils,
      queryClient,
    };
  };

  it('検索結果のアバターがフォールバック画像を使用する', async () => {
    const { container, queryClient } = renderWithQueryClient('alice');

    await waitFor(() => {
      expect(searchUsersSpy).toHaveBeenCalledWith('alice', 24);
    });

    await waitFor(() => {
      expect(screen.getByText('Alice')).toBeInTheDocument();
    });

    await waitFor(() => {
      expect(
        container.querySelector('[data-slot="avatar-image"]') as HTMLImageElement | null,
      ).not.toBeNull();
    });
    expect(avatarImageSources[0]).toBe(resolveUserAvatarSrc({ picture: '' }));

    queryClient.clear();
  });
});
