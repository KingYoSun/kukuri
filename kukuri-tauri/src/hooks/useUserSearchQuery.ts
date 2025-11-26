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
export const MIN_USER_SEARCH_QUERY_LENGTH = 2;
export const MAX_USER_SEARCH_QUERY_LENGTH = 64;

const MAX_QUERY_LENGTH = MAX_USER_SEARCH_QUERY_LENGTH;

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

export type HelperSearchKind = 'mention' | 'hashtag';

export interface HelperSearchDescriptor {
  kind: HelperSearchKind;
  term: string;
  rawQuery: string;
}

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
  helperSearch: HelperSearchDescriptor | null;
  allowIncompleteActive: boolean;
}

export function useUserSearchQuery(
  query: string,
  options?: UseUserSearchQueryOptions,
): UseUserSearchQueryResult {
  const minLength = options?.minLength ?? MIN_USER_SEARCH_QUERY_LENGTH;
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

  const helperSearch = useMemo(() => detectHelperSearch(clampedQuery), [clampedQuery]);
  const helperTermLength = helperSearch?.term.length ?? 0;
  const effectiveQueryLength = helperSearch ? helperTermLength : clampedQuery.length;
  const allowIncompleteActive = Boolean(
    helperSearch && helperTermLength > 0 && helperTermLength < minLength,
  );

  const meetsMinLength = effectiveQueryLength >= minLength;
  const queryEnabled = (meetsMinLength || allowIncompleteActive) && cooldownSeconds === null;

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
        allowIncomplete: allowIncompleteActive,
      }),
    initialPageParam: null,
    getNextPageParam: (lastPage) => lastPage.nextCursor ?? undefined,
    enabled: queryEnabled,
    refetchOnWindowFocus: false,
    retry: false,
  });

  useEffect(() => {
    if (queryResult.isSuccess && (meetsMinLength || allowIncompleteActive)) {
      setStaleData(queryResult.data ?? null);
    }
  }, [allowIncompleteActive, meetsMinLength, queryResult.data, queryResult.isSuccess]);

  useEffect(() => {
    if (clampedQuery.length === 0) {
      setErrorKey(null);
      return;
    }
    if (!meetsMinLength && !allowIncompleteActive) {
      setErrorKey('UserSearch.invalid_query');
    } else if (errorKey === 'UserSearch.invalid_query') {
      setErrorKey(null);
    }
  }, [allowIncompleteActive, clampedQuery, errorKey, meetsMinLength]);

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

  const shouldUseStaleData =
    !meetsMinLength && (!allowIncompleteActive || queryResult.isLoading || queryResult.isFetching);

  const activeData: InfiniteData<SearchUsersResponseDto, string | null> | null =
    queryResult.data ?? (shouldUseStaleData ? staleData : null);

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
    meetsMinLength,
    cooldownSeconds,
    debouncedQuery,
    dataAvailable: flattenedResults.length > 0,
    queryResult,
    allowIncomplete: allowIncompleteActive,
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
    helperSearch,
    allowIncompleteActive,
  };
}

async function fetchSearchPage({
  query,
  cursor,
  pageSize,
  viewerNpub,
  sort,
  allowIncomplete,
}: {
  query: string;
  cursor: string | null;
  pageSize: number;
  viewerNpub: string | null;
  sort: UserSearchSort;
  allowIncomplete: boolean;
}): Promise<SearchUsersResponseDto> {
  const resolveE2EFixturePage = (): SearchUsersResponseDto | null => {
    if (import.meta.env.VITE_ENABLE_E2E !== 'true' || typeof window === 'undefined') {
      return null;
    }
    const fixture = (window as unknown as { __E2E_USER_SEARCH_FIXTURE__?: SearchUsersResponseDto })
      .__E2E_USER_SEARCH_FIXTURE__;
    if (!fixture || !Array.isArray(fixture.items) || fixture.items.length === 0) {
      return null;
    }

    const parsedCursor =
      typeof cursor === 'number'
        ? cursor
        : typeof cursor === 'string' && cursor.length > 0
          ? Number.parseInt(cursor, 10)
          : 0;
    const startIndex =
      Number.isNaN(parsedCursor) || !Number.isFinite(parsedCursor) || parsedCursor < 0
        ? 0
        : parsedCursor;
    const effectivePageSize =
      Number.isFinite(pageSize) && pageSize > 0 ? pageSize : fixture.items.length;

    const start = Math.min(startIndex, fixture.items.length);
    const end = Math.min(start + effectivePageSize, fixture.items.length);
    const pageItems = fixture.items.slice(start, end);
    const nextCursor = end < fixture.items.length ? String(end) : null;

    return {
      items: pageItems,
      nextCursor,
      hasMore: nextCursor !== null,
      totalCount: fixture.totalCount ?? fixture.items.length,
      tookMs: fixture.tookMs ?? 1,
    };
  };

  const e2eFixturePage = resolveE2EFixturePage();

  let response: SearchUsersResponseDto | null = null;
  try {
    response = await TauriApi.searchUsers({
      query,
      cursor,
      limit: pageSize,
      sort,
      viewerNpub,
      allowIncomplete,
    });
  } catch (error) {
    if (error instanceof TauriCommandError && error.code === 'RATE_LIMITED') {
      throw error;
    }

    const rateLimitedPayload =
      (error as {
        code?: string | null;
        RateLimited?: { retry_after_seconds?: number | string | null };
        retry_after_seconds?: number | string | null;
      }) ?? null;
    const rateLimitedDetails = rateLimitedPayload?.RateLimited ?? null;
    const isRateLimited =
      rateLimitedPayload?.code === 'RATE_LIMITED' ||
      rateLimitedPayload?.code === 'RateLimited' ||
      Boolean(rateLimitedDetails);

    if (isRateLimited) {
      const retryAfterSeconds =
        Number(
          rateLimitedDetails?.retry_after_seconds ?? rateLimitedPayload?.retry_after_seconds,
        ) || null;
      throw new TauriCommandError(
        error instanceof Error ? error.message : 'Rate limited',
        'RATE_LIMITED',
        retryAfterSeconds !== null ? { retry_after_seconds: retryAfterSeconds } : undefined,
      );
    }
    if (e2eFixturePage) {
      return e2eFixturePage;
    }
    throw error;
  }

  if (e2eFixturePage) {
    return e2eFixturePage;
  }

  if (response && response.items.length > 0) {
    return response;
  }

  if (e2eFixturePage) {
    return e2eFixturePage;
  }

  return (
    response ?? {
      items: [],
      nextCursor: null,
      hasMore: false,
      totalCount: 0,
      tookMs: 0,
    }
  );
}

function sanitizeQuery(raw: string): string {
  return raw.replace(CONTROL_CHARS, '').replace(/\s+/g, ' ').trim();
}

export function sanitizeUserSearchQuery(raw: string): string {
  return sanitizeQuery(raw);
}

export function detectUserSearchHelper(query: string): HelperSearchDescriptor | null {
  return detectHelperSearch(query);
}

function deriveStatus({
  clampedQuery,
  meetsMinLength,
  cooldownSeconds,
  debouncedQuery,
  dataAvailable,
  queryResult,
  allowIncomplete,
}: {
  clampedQuery: string;
  meetsMinLength: boolean;
  cooldownSeconds: number | null;
  debouncedQuery: string;
  dataAvailable: boolean;
  queryResult: UseInfiniteQueryResult<InfiniteData<SearchUsersResponseDto, string | null>, Error>;
  allowIncomplete: boolean;
}): UserSearchStatus {
  if (!clampedQuery.length) {
    return 'idle';
  }
  if (!meetsMinLength && !allowIncomplete) {
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

function detectHelperSearch(query: string): HelperSearchDescriptor | null {
  const trimmed = query.trim();
  if (trimmed.length <= 1) {
    return null;
  }

  const startsWithMention = trimmed.startsWith('@');
  const endsWithMention = trimmed.endsWith('@');
  if (startsWithMention || endsWithMention) {
    const term = startsWithMention ? trimmed.slice(1) : trimmed.slice(0, -1);
    const normalized = term.trim();
    if (!normalized.length) {
      return null;
    }
    return { kind: 'mention', term: normalized, rawQuery: trimmed };
  }

  const startsWithTag = trimmed.startsWith('#');
  const endsWithTag = trimmed.endsWith('#');
  if (startsWithTag || endsWithTag) {
    const term = startsWithTag ? trimmed.slice(1) : trimmed.slice(0, -1);
    const normalized = term.trim();
    if (!normalized.length) {
      return null;
    }
    return { kind: 'hashtag', term: normalized, rawQuery: trimmed };
  }

  return null;
}
