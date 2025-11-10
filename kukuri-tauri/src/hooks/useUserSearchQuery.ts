import {
  useInfiniteQuery,
  type InfiniteData,
  type UseInfiniteQueryResult,
} from '@tanstack/react-query';
import { useCallback, useEffect, useMemo, useState } from 'react';

import { TauriApi, type SearchUsersResponse as SearchUsersResponseDto } from '@/lib/api/tauri';
import { TauriCommandError } from '@/lib/api/tauriClient';
import { errorHandler } from '@/lib/errorHandler';
import { mapUserProfileToUser } from '@/lib/profile/profileMapper';
import type { Profile } from '@/stores/types';
import { useDebounce } from './useDebounce';

// eslint-disable-next-line no-control-regex
const CONTROL_CHARS = /[\u0000-\u001F\u007F]/g;
const MAX_QUERY_LENGTH = 64;

export type UserSearchSort = 'relevance' | 'recency';

type UserSearchQueryKey = ['user-search', string, string | null, UserSearchSort];

export type UserSearchStatus =
  | 'idle'
  | 'typing'
  | 'ready'
  | 'loading'
  | 'success'
  | 'empty'
  | 'error'
  | 'rateLimited';

export type UserSearchErrorKey =
  | 'UserSearch.invalid_query'
  | 'UserSearch.fetch_failed'
  | 'UserSearch.rate_limited';

interface UseUserSearchQueryOptions {
  minLength?: number;
  pageSize?: number;
  viewerNpub?: string | null;
  sort?: UserSearchSort;
}

interface UseUserSearchQueryResult {
  status: UserSearchStatus;
  sanitizedQuery: string;
  results: Profile[];
  totalCount: number;
  tookMs: number;
  errorKey: UserSearchErrorKey | null;
  retryAfterSeconds: number | null;
  hasNextPage: boolean;
  isFetching: boolean;
  isFetchingNextPage: boolean;
  fetchNextPage: () => Promise<void>;
  onRetry: () => Promise<void>;
}

export function useUserSearchQuery(
  query: string,
  options?: UseUserSearchQueryOptions,
): UseUserSearchQueryResult {
  const minLength = options?.minLength ?? 2;
  const pageSize = options?.pageSize ?? 24;
  const viewerNpub = options?.viewerNpub ?? null;
  const sort: UserSearchSort = options?.sort ?? 'relevance';

  const sanitizedQuery = useMemo(() => sanitizeQuery(query), [query]);
  const clampedQuery =
    sanitizedQuery.length > MAX_QUERY_LENGTH
      ? sanitizedQuery.slice(0, MAX_QUERY_LENGTH)
      : sanitizedQuery;
  const debouncedQuery = useDebounce<string>(clampedQuery, 300);

  const [cooldownSeconds, setCooldownSeconds] = useState<number | null>(null);
  const [errorKey, setErrorKey] = useState<UserSearchErrorKey | null>(null);
  const [staleData, setStaleData] = useState<InfiniteData<
    SearchUsersResponseDto,
    string | null
  > | null>(null);

  const queryEnabled = debouncedQuery.length >= minLength && cooldownSeconds === null;

  const queryResult = useInfiniteQuery<
    SearchUsersResponseDto,
    Error,
    InfiniteData<SearchUsersResponseDto, string | null>,
    UserSearchQueryKey,
    string | null
  >({
    queryKey: ['user-search', debouncedQuery, viewerNpub, sort],
    queryFn: ({ pageParam }) =>
      fetchSearchPage({
        query: debouncedQuery,
        cursor: pageParam ?? null,
        pageSize,
        viewerNpub,
        sort,
      }),
    initialPageParam: null,
    getNextPageParam: (lastPage) => lastPage.nextCursor ?? undefined,
    enabled: queryEnabled,
    refetchOnWindowFocus: false,
    retry: false,
  });

  useEffect(() => {
    if (queryResult.isSuccess && debouncedQuery.length >= minLength) {
      setStaleData(queryResult.data ?? null);
    }
  }, [debouncedQuery, minLength, queryResult.data, queryResult.isSuccess]);

  useEffect(() => {
    if (clampedQuery.length === 0) {
      setErrorKey(null);
      return;
    }
    if (clampedQuery.length < minLength) {
      setErrorKey('UserSearch.invalid_query');
    } else if (errorKey === 'UserSearch.invalid_query') {
      setErrorKey(null);
    }
  }, [clampedQuery, minLength]);

  useEffect(() => {
    const error = queryResult.error;
    if (!error) {
      if (cooldownSeconds === null && errorKey !== 'UserSearch.invalid_query') {
        setErrorKey(null);
      }
      return;
    }

    if (error instanceof TauriCommandError && error.code === 'RATE_LIMITED') {
      const retryAfter = Number(error.details?.retry_after_seconds) || 5;
      setCooldownSeconds(retryAfter);
      setErrorKey('UserSearch.rate_limited');
      return;
    }

    errorHandler.log('UserSearch.fetch_failed', error, {
      context: 'useUserSearchQuery',
      metadata: { query: debouncedQuery },
    });
    setErrorKey('UserSearch.fetch_failed');
  }, [queryResult.error, cooldownSeconds, errorKey, debouncedQuery]);

  useEffect(() => {
    if (cooldownSeconds === null) {
      return;
    }
    if (cooldownSeconds <= 0) {
      setCooldownSeconds(null);
      setErrorKey(null);
      void queryResult.refetch();
      return;
    }
    const timer = setTimeout(() => {
      setCooldownSeconds((value) => (value === null ? null : Math.max(value - 1, 0)));
    }, 1000);
    return () => clearTimeout(timer);
  }, [cooldownSeconds, queryResult]);

  const activeData: InfiniteData<SearchUsersResponseDto, string | null> | null =
    queryResult.data ?? (clampedQuery.length < minLength ? staleData : null);

  const flattenedResults = useMemo(() => {
    if (!activeData) {
      return [];
    }
    return activeData.pages.flatMap((page) =>
      page.items.map((profile) => mapUserProfileToUser(profile)),
    );
  }, [activeData]);

  const totalCount = activeData?.pages?.[0]?.totalCount ?? 0;
  const tookMs = activeData?.pages?.[0]?.tookMs ?? 0;

  const hasNextPage = Boolean(queryResult.hasNextPage);
  const isFetching = queryResult.isFetching && !queryResult.isFetchingNextPage;
  const isFetchingNextPage = queryResult.isFetchingNextPage;

  const fetchNextPage = useCallback(async () => {
    if (!hasNextPage || queryResult.isFetchingNextPage) {
      return;
    }
    await queryResult.fetchNextPage();
  }, [hasNextPage, queryResult]);

  const handleRetry = useCallback(async () => {
    if (cooldownSeconds !== null && cooldownSeconds > 0) {
      return;
    }
    setCooldownSeconds(null);
    setErrorKey(null);
    await queryResult.refetch();
  }, [cooldownSeconds, queryResult]);

  const status = deriveStatus({
    clampedQuery,
    minLength,
    cooldownSeconds,
    debouncedQuery,
    dataAvailable: flattenedResults.length > 0,
    queryResult,
  });

  return {
    status,
    sanitizedQuery: clampedQuery,
    results: flattenedResults,
    totalCount,
    tookMs,
    errorKey: errorKey ?? null,
    retryAfterSeconds: cooldownSeconds,
    hasNextPage,
    isFetching,
    isFetchingNextPage,
    fetchNextPage,
    onRetry: handleRetry,
  };
}

async function fetchSearchPage({
  query,
  cursor,
  pageSize,
  viewerNpub,
  sort,
}: {
  query: string;
  cursor: string | null;
  pageSize: number;
  viewerNpub: string | null;
  sort: UserSearchSort;
}): Promise<SearchUsersResponseDto> {
  return await TauriApi.searchUsers({
    query,
    cursor,
    limit: pageSize,
    sort,
    viewerNpub,
  });
}

function sanitizeQuery(raw: string): string {
  return raw.replace(CONTROL_CHARS, '').replace(/\s+/g, ' ').trim();
}

function deriveStatus({
  clampedQuery,
  minLength,
  cooldownSeconds,
  debouncedQuery,
  dataAvailable,
  queryResult,
}: {
  clampedQuery: string;
  minLength: number;
  cooldownSeconds: number | null;
  debouncedQuery: string;
  dataAvailable: boolean;
  queryResult: UseInfiniteQueryResult<InfiniteData<SearchUsersResponseDto, string | null>, Error>;
}): UserSearchStatus {
  if (!clampedQuery.length) {
    return 'idle';
  }
  if (clampedQuery.length < minLength) {
    return 'typing';
  }
  if (cooldownSeconds !== null) {
    return 'rateLimited';
  }
  if (debouncedQuery !== clampedQuery) {
    return 'ready';
  }
  if (queryResult.isLoading && !dataAvailable) {
    return 'loading';
  }
  if (queryResult.isError) {
    return 'error';
  }
  if (!dataAvailable) {
    return 'empty';
  }
  return 'success';
}
