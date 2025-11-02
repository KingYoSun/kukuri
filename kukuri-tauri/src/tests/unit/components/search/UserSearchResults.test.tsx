import React from 'react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen, waitFor } from '@testing-library/react';
import { vi } from 'vitest';

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

import { UserSearchResults } from '@/components/search/UserSearchResults';
import { resolveUserAvatarSrc } from '@/lib/profile/avatarDisplay';

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

describe('UserSearchResults', () => {
  beforeEach(() => {
    avatarImageSources.length = 0;
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
