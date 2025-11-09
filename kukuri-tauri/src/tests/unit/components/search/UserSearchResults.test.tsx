import React from 'react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import { vi, describe, beforeEach, afterEach, it, type Mock } from 'vitest';

import { UserSearchResults } from '@/components/search/UserSearchResults';
import { useAuthStore } from '@/stores/authStore';
import { useUserSearchQuery } from '@/hooks/useUserSearchQuery';
import { TauriApi } from '@/lib/api/tauri';

vi.mock('@/hooks/useUserSearchQuery');
vi.mock('@/lib/api/tauri', () => ({
  TauriApi: {
    getFollowing: vi
      .fn()
      .mockResolvedValue({ items: [], next_cursor: null, has_more: false, total_count: 0 }),
    followUser: vi.fn(),
    unfollowUser: vi.fn(),
  },
}));

vi.mock('@tanstack/react-router', () => ({
  Link: ({
    children,
    to,
  }: {
    children: React.ReactNode;
    to: string;
    params?: Record<string, unknown>;
  }) => (
    <a href={typeof to === 'string' ? to : '#'} data-testid="router-link">
      {children}
    </a>
  ),
}));

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
  }: React.ImgHTMLAttributes<HTMLImageElement> & { src?: string | undefined }) => (
    <img data-slot="avatar-image" src={src || ''} {...props} />
  ),
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
  const useSearchQueryMock = useUserSearchQuery as unknown as Mock;
  const getFollowingMock = TauriApi.getFollowing as Mock;

  beforeEach(() => {
    vi.clearAllMocks();
    useAuthStore.setState({
      currentUser: {
        npub: 'viewer',
        pubkey: 'viewer',
        id: 'viewer',
        displayName: 'Viewer',
        name: 'Viewer',
        about: '',
        picture: '',
        nip05: '',
        publicProfile: true,
        showOnlineStatus: false,
      } as any,
    });
    useSearchQueryMock.mockReturnValue({
      status: 'success',
      sanitizedQuery: 'alice',
      results: [
        {
          id: 'pubkey1',
          pubkey: 'pubkey1',
          npub: 'npub1',
          name: 'Alice',
          displayName: 'Alice',
          picture: '',
          about: '',
          nip05: '',
          publicProfile: true,
          showOnlineStatus: false,
        },
      ],
      totalCount: 1,
      tookMs: 10,
      hasNextPage: false,
      isFetching: false,
      isFetchingNextPage: false,
      fetchNextPage: vi.fn(),
      errorKey: null,
      retryAfterSeconds: null,
      onRetry: vi.fn(),
    });

    getFollowingMock.mockResolvedValue({
      items: [],
      next_cursor: null,
      has_more: false,
      total_count: 0,
    });
  });

  afterEach(() => {
    useAuthStore.setState({ currentUser: null } as any);
  });

  const renderWithClient = (ui: React.ReactElement) => {
    const queryClient = new QueryClient({
      defaultOptions: {
        queries: {
          retry: false,
        },
      },
    });

    return render(<QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>);
  };

  it('renders idle state when query is empty', () => {
    useSearchQueryMock.mockReturnValueOnce({
      status: 'idle',
      sanitizedQuery: '',
      results: [],
      totalCount: 0,
      tookMs: 0,
      hasNextPage: false,
      isFetching: false,
      isFetchingNextPage: false,
      fetchNextPage: vi.fn(),
      errorKey: null,
      retryAfterSeconds: null,
      onRetry: vi.fn(),
    });

    renderWithClient(<UserSearchResults query="" />);

    expect(screen.getByText('検索キーワードを入力してください。')).toBeInTheDocument();
  });

  it('renders search results when hook returns data', () => {
    renderWithClient(<UserSearchResults query="alice" />);
    expect(screen.getByText('Alice')).toBeInTheDocument();
    expect(screen.getByText('1 件ヒット')).toBeInTheDocument();
  });

  it('renders empty state when no users are found', () => {
    useSearchQueryMock.mockReturnValueOnce({
      status: 'empty',
      sanitizedQuery: 'zzz',
      results: [],
      totalCount: 0,
      tookMs: 5,
      hasNextPage: false,
      isFetching: false,
      isFetchingNextPage: false,
      fetchNextPage: vi.fn(),
      errorKey: null,
      retryAfterSeconds: null,
      onRetry: vi.fn(),
    });

    renderWithClient(<UserSearchResults query="zzz" />);
    expect(screen.getByText('該当するユーザーが見つかりませんでした')).toBeInTheDocument();
  });
});
