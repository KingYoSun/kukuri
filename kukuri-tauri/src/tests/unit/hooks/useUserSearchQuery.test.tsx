import React from 'react';
import { renderHook, act, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { vi, describe, it, expect } from 'vitest';

import { useUserSearchQuery } from '@/hooks/useUserSearchQuery';
import { TauriApi } from '@/lib/api/tauri';
import { TauriCommandError } from '@/lib/api/tauriClient';

vi.mock('@/lib/api/tauri', () => ({
  TauriApi: {
    searchUsers: vi.fn(),
  },
}));

const searchUsersMock = TauriApi.searchUsers as ReturnType<typeof vi.fn>;
const waitForDebounce = () =>
  act(async () => {
    await new Promise((resolve) => setTimeout(resolve, 350));
  });

const createWrapper = () => {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
      },
    },
  });

  return function Wrapper({ children }: { children: React.ReactNode }) {
    return <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>;
  };
};

describe('useUserSearchQuery', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('fetches users when query length meets minimum', async () => {
    searchUsersMock.mockResolvedValueOnce({
      items: [
        {
          npub: 'npub1alice',
          pubkey: 'pubkey1',
          name: 'Alice',
          display_name: 'Alice',
          about: '',
          picture: '',
          banner: null,
          website: null,
          nip05: null,
          is_profile_public: true,
          show_online_status: false,
        },
      ],
      nextCursor: null,
      hasMore: false,
      totalCount: 1,
      tookMs: 5,
    });

    const { result } = renderHook(({ value }) => useUserSearchQuery(value), {
      initialProps: { value: 'alice' },
      wrapper: createWrapper(),
    });

    await waitForDebounce();

    await waitFor(() => {
      expect(result.current.results).toHaveLength(1);
    });
    expect(result.current.status).toBe('success');
    expect(searchUsersMock).toHaveBeenCalledWith({
      query: 'alice',
      cursor: null,
      limit: 24,
      sort: 'relevance',
      viewerNpub: null,
      allowIncomplete: false,
    });
  });

  it('does not search when query length is below minimum', async () => {
    const { result } = renderHook(({ value }) => useUserSearchQuery(value), {
      initialProps: { value: 'a' },
      wrapper: createWrapper(),
    });

    await waitForDebounce();

    expect(result.current.status).toBe('typing');
    expect(searchUsersMock).not.toHaveBeenCalled();
  });

  it('enables allow_incomplete fallback for helper queries', async () => {
    searchUsersMock.mockResolvedValueOnce({
      items: [],
      nextCursor: null,
      hasMore: false,
      totalCount: 0,
      tookMs: 1,
    });

    const { result } = renderHook(({ value }) => useUserSearchQuery(value), {
      initialProps: { value: '@a' },
      wrapper: createWrapper(),
    });

    await waitForDebounce();

    await waitFor(() => {
      expect(searchUsersMock).toHaveBeenCalledWith(
        expect.objectContaining({ allowIncomplete: true }),
      );
    });
    expect(result.current.allowIncompleteActive).toBe(true);
    expect(result.current.helperSearch).toEqual(
      expect.objectContaining({ kind: 'mention', term: 'a' }),
    );
  });

  it('exposes rate limit state when API returns RATE_LIMITED', async () => {
    searchUsersMock.mockRejectedValueOnce(
      new TauriCommandError('rate limited', 'RATE_LIMITED', { retry_after_seconds: 2 }),
    );

    const { result } = renderHook(({ value }) => useUserSearchQuery(value), {
      initialProps: { value: 'alice' },
      wrapper: createWrapper(),
    });

    await waitForDebounce();

    await waitFor(() => {
      expect(result.current.status).toBe('rateLimited');
      expect(result.current.retryAfterSeconds).toBeGreaterThan(0);
    });
  });

  it('fetches next page when fetchNextPage is called', async () => {
    searchUsersMock
      .mockResolvedValueOnce({
        items: [
          {
            npub: 'npub1alice',
            pubkey: 'pubkey1',
            name: 'Alice',
            display_name: 'Alice',
            about: '',
            picture: '',
            banner: null,
            website: null,
            nip05: null,
            is_profile_public: true,
            show_online_status: false,
          },
        ],
        nextCursor: 'cursor123',
        hasMore: true,
        totalCount: 2,
        tookMs: 5,
      })
      .mockResolvedValueOnce({
        items: [
          {
            npub: 'npub1bob',
            pubkey: 'pubkey2',
            name: 'Bob',
            display_name: 'Bob',
            about: '',
            picture: '',
            banner: null,
            website: null,
            nip05: null,
            is_profile_public: true,
            show_online_status: false,
          },
        ],
        nextCursor: null,
        hasMore: false,
        totalCount: 2,
        tookMs: 5,
      });

    const { result } = renderHook(({ value }) => useUserSearchQuery(value), {
      initialProps: { value: 'alice' },
      wrapper: createWrapper(),
    });

    await waitForDebounce();

    await waitFor(() => expect(result.current.results).toHaveLength(1));

    await act(async () => {
      await result.current.fetchNextPage();
    });

    await waitFor(() => expect(result.current.results).toHaveLength(2));
  });

  it('respects custom sort option changes', async () => {
    searchUsersMock
      .mockResolvedValueOnce({
        items: [
          {
            npub: 'npub1alice',
            pubkey: 'pubkey1',
            name: 'Alice',
            display_name: 'Alice',
            about: '',
            picture: '',
            banner: null,
            website: null,
            nip05: null,
            is_profile_public: true,
            show_online_status: false,
          },
        ],
        nextCursor: null,
        hasMore: false,
        totalCount: 1,
        tookMs: 5,
      })
      .mockResolvedValueOnce({
        items: [
          {
            npub: 'npub1bob',
            pubkey: 'pubkey2',
            name: 'Bob',
            display_name: 'Bob',
            about: '',
            picture: '',
            banner: null,
            website: null,
            nip05: null,
            is_profile_public: true,
            show_online_status: false,
          },
        ],
        nextCursor: null,
        hasMore: false,
        totalCount: 1,
        tookMs: 4,
      });

    const { rerender } = renderHook(({ value, sort }) => useUserSearchQuery(value, { sort }), {
      initialProps: { value: 'alice', sort: 'relevance' as const },
      wrapper: createWrapper(),
    });

    await waitForDebounce();
    await waitFor(() => {
      expect(searchUsersMock).toHaveBeenCalledWith(
        expect.objectContaining({ sort: 'relevance', allowIncomplete: false }),
      );
    });

    rerender({ value: 'alice', sort: 'recency' });
    await waitForDebounce();

    await waitFor(() => {
      expect(searchUsersMock).toHaveBeenLastCalledWith(
        expect.objectContaining({ sort: 'recency', allowIncomplete: false }),
      );
    });
  });
});
