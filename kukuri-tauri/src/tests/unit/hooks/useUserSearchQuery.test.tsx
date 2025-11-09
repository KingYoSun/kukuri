import React from 'react';
import { renderHook, act, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { vi, describe, beforeEach, afterEach, it, expect } from 'vitest';

import { useUserSearchQuery } from '@/hooks/useUserSearchQuery';
import { TauriApi } from '@/lib/api/tauri';
import { TauriCommandError } from '@/lib/api/tauriClient';

vi.mock('@/lib/api/tauri', () => ({
  TauriApi: {
    searchUsers: vi.fn(),
  },
}));

const searchUsersMock = TauriApi.searchUsers as ReturnType<typeof vi.fn>;

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
    vi.useFakeTimers();
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.useRealTimers();
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

    const { result } = renderHook(
      ({ value }) => useUserSearchQuery(value),
      {
        initialProps: { value: 'alice' },
        wrapper: createWrapper(),
      },
    );

    await act(async () => {
      vi.advanceTimersByTime(350);
    });

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
    });
  });

  it('does not search when query length is below minimum', async () => {
    const { result } = renderHook(
      ({ value }) => useUserSearchQuery(value),
      {
        initialProps: { value: 'a' },
        wrapper: createWrapper(),
      },
    );

    await act(async () => {
      vi.advanceTimersByTime(350);
    });

    expect(result.current.status).toBe('typing');
    expect(searchUsersMock).not.toHaveBeenCalled();
  });

  it('exposes rate limit state when API returns RATE_LIMITED', async () => {
    searchUsersMock.mockRejectedValueOnce(
      new TauriCommandError('rate limited', 'RATE_LIMITED', { retry_after_seconds: 2 }),
    );

    const { result } = renderHook(
      ({ value }) => useUserSearchQuery(value),
      {
        initialProps: { value: 'alice' },
        wrapper: createWrapper(),
      },
    );

    await act(async () => {
      vi.advanceTimersByTime(350);
    });

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

    const { result } = renderHook(
      ({ value }) => useUserSearchQuery(value),
      {
        initialProps: { value: 'alice' },
        wrapper: createWrapper(),
      },
    );

    await act(async () => {
      vi.advanceTimersByTime(350);
    });

    await waitFor(() => expect(result.current.results).toHaveLength(1));

    await act(async () => {
      await result.current.fetchNextPage();
    });

    await waitFor(() => expect(result.current.results).toHaveLength(2));
  });
});
