import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { KeyboardEvent } from 'react';
import { useTranslation } from 'react-i18next';
import { useInfiniteQuery, type InfiniteData } from '@tanstack/react-query';
import { useVirtualizer } from '@tanstack/react-virtual';
import { formatDistanceToNow } from 'date-fns';
import { Loader2, Search as SearchIcon } from 'lucide-react';
import { getDateFnsLocale } from '@/i18n';

import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Badge } from '@/components/ui/badge';
import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar';
import { useDebounce } from '@/hooks/useDebounce';
import { useDirectMessageStore } from '@/stores/directMessageStore';
import type { Profile } from '@/stores';
import { useAuthStore } from '@/stores/authStore';
import { errorHandler } from '@/lib/errorHandler';
import { mapUserProfileToUser } from '@/lib/profile/profileMapper';
import {
  TauriApi,
  type DirectMessageConversationList,
  type DirectMessageConversationSummary,
  type DirectMessageItem,
} from '@/lib/api/tauri';
import { cn } from '@/lib/utils';

const formatRelativeTime = (timestamp: number | null | undefined) => {
  if (!timestamp) {
    return { display: null, helper: null };
  }
  const date = new Date(timestamp);
  return {
    display: formatDistanceToNow(date, { addSuffix: true, locale: getDateFnsLocale() }),
    helper: date.toLocaleString(),
  };
};

const formatNpub = (npub: string) => {
  if (npub.length <= 16) {
    return npub;
  }
  return `${npub.slice(0, 8)}…${npub.slice(-6)}`;
};

type ConversationEntry = {
  npub: string;
  lastMessage: DirectMessageItem | null;
  unread: number;
  lastReadAt: number;
};

const EMPTY_CONVERSATION_PAGES: DirectMessageConversationList[] = [];

export function DirectMessageInbox() {
  const { t } = useTranslation();
  const currentUser = useAuthStore((state) => state.currentUser);
  const isInboxOpen = useDirectMessageStore((state) => state.isInboxOpen);
  const closeInbox = useDirectMessageStore((state) => state.closeInbox);
  const openDialog = useDirectMessageStore((state) => state.openDialog);
  const activeConversationNpub = useDirectMessageStore((state) => state.activeConversationNpub);
  const markConversationAsRead = useDirectMessageStore((state) => state.markConversationAsRead);
  const conversationMessages = useDirectMessageStore((state) => state.conversations);
  const conversationUnreadCounts = useDirectMessageStore((state) => state.unreadCounts);
  const conversationReadTimestamps = useDirectMessageStore(
    (state) => state.conversationReadTimestamps,
  );
  const [targetNpub, setTargetNpub] = useState('');
  const [validationError, setValidationError] = useState<string | null>(null);
  const [searchResults, setSearchResults] = useState<Profile[]>([]);
  const [isSearching, setIsSearching] = useState(false);
  const [searchError, setSearchError] = useState<string | null>(null);
  const [conversationQuery, setConversationQuery] = useState('');
  const debouncedRecipientQuery = useDebounce(targetNpub.trim(), 300);
  const normalizedConversationQuery = conversationQuery.trim().toLowerCase();

  const conversationsQuery = useInfiniteQuery<
    DirectMessageConversationList,
    Error,
    InfiniteData<DirectMessageConversationList, string | null>,
    ['direct-message-conversations'],
    string | null
  >({
    queryKey: ['direct-message-conversations'],
    enabled: isInboxOpen,
    retry: false,
    initialPageParam: null,
    queryFn: async ({ pageParam }) => {
      return await TauriApi.listDirectMessageConversations({
        cursor: pageParam ?? null,
        limit: 30,
      });
    },
    getNextPageParam: (lastPage) =>
      lastPage.hasMore ? (lastPage.nextCursor ?? undefined) : undefined,
  });

  const conversationPages = conversationsQuery.data?.pages ?? EMPTY_CONVERSATION_PAGES;
  const apiConversationEntries = useMemo<ConversationEntry[]>(() => {
    return conversationPages
      .flatMap((page: DirectMessageConversationList) => page.items)
      .map((item: DirectMessageConversationSummary) => ({
        npub: item.conversationNpub,
        lastMessage: item.lastMessage,
        unread: item.unreadCount,
        lastReadAt: item.lastReadAt,
      }));
  }, [conversationPages]);
  const storeConversationEntries = useMemo<ConversationEntry[]>(() => {
    const entries: ConversationEntry[] = [];
    const candidates = new Set<string>([
      ...Object.keys(conversationMessages),
      ...Object.keys(conversationUnreadCounts),
    ]);

    for (const npub of candidates) {
      const messages = conversationMessages[npub] ?? [];
      const lastMessage = messages.length > 0 ? messages[messages.length - 1] : null;
      entries.push({
        npub,
        lastMessage: lastMessage
          ? {
              eventId: lastMessage.eventId,
              clientMessageId: lastMessage.clientMessageId ?? null,
              senderNpub: lastMessage.senderNpub,
              recipientNpub: lastMessage.recipientNpub,
              content: lastMessage.content,
              createdAt: lastMessage.createdAt,
              delivered: lastMessage.status !== 'pending',
            }
          : null,
        unread: conversationUnreadCounts[npub] ?? 0,
        lastReadAt: conversationReadTimestamps[npub] ?? 0,
      });
    }

    return entries
      .filter((entry) => entry.lastMessage !== null || entry.unread > 0)
      .sort((a, b) => (b.lastMessage?.createdAt ?? 0) - (a.lastMessage?.createdAt ?? 0));
  }, [conversationMessages, conversationUnreadCounts, conversationReadTimestamps]);
  const conversationEntries = useMemo<ConversationEntry[]>(() => {
    const merged = new Map<string, ConversationEntry>();
    for (const entry of apiConversationEntries) {
      merged.set(entry.npub, entry);
    }
    for (const entry of storeConversationEntries) {
      if (!merged.has(entry.npub)) {
        merged.set(entry.npub, entry);
      }
    }
    return Array.from(merged.values());
  }, [apiConversationEntries, storeConversationEntries]);
  const filteredConversationEntries = useMemo(() => {
    if (!normalizedConversationQuery) {
      return conversationEntries;
    }
    return conversationEntries.filter((entry) => {
      const content = entry.lastMessage?.content?.toLowerCase() ?? '';
      return (
        entry.npub.toLowerCase().includes(normalizedConversationQuery) ||
        content.includes(normalizedConversationQuery)
      );
    });
  }, [conversationEntries, normalizedConversationQuery]);
  const hasConversations = conversationEntries.length > 0;
  const hasFilteredConversations = filteredConversationEntries.length > 0;
  const conversationListRef = useRef<HTMLDivElement | null>(null);
  const {
    isLoading: isConversationsLoading,
    isError: isConversationsError,
    error: conversationsError,
    fetchNextPage,
    hasNextPage,
    isFetchingNextPage,
    refetch: refetchConversations,
  } = conversationsQuery;
  const showLoadingState = isConversationsLoading && !hasConversations;
  const showErrorState = isConversationsError && !hasConversations;
  const getConversationItemKey = useCallback(
    (index: number) => filteredConversationEntries[index]?.npub ?? `loader-${index}`,
    [filteredConversationEntries],
  );
  const rowVirtualizer = useVirtualizer({
    count: hasNextPage
      ? filteredConversationEntries.length + 1
      : filteredConversationEntries.length,
    getScrollElement: () => conversationListRef.current,
    estimateSize: () => 88,
    overscan: 12,
    getItemKey: getConversationItemKey,
  });
  const virtualItems = rowVirtualizer.getVirtualItems();
  const shouldRenderFallbackList =
    filteredConversationEntries.length <= 20 || virtualItems.length === 0;
  const autoCompleteConversationNpub = filteredConversationEntries[0]?.npub ?? null;

  useEffect(() => {
    if (!isInboxOpen || !activeConversationNpub) {
      return;
    }
    const index = filteredConversationEntries.findIndex(
      (entry) => entry.npub === activeConversationNpub,
    );
    if (index >= 0) {
      rowVirtualizer.scrollToIndex(index, { align: 'center' });
    }
  }, [isInboxOpen, activeConversationNpub, filteredConversationEntries, rowVirtualizer]);

  useEffect(() => {
    if (!hasNextPage || isFetchingNextPage) {
      return;
    }
    const virtualItems = rowVirtualizer.getVirtualItems();
    if (virtualItems.length === 0) {
      return;
    }
    const lastItem = virtualItems[virtualItems.length - 1];
    if (lastItem.index >= filteredConversationEntries.length) {
      void fetchNextPage();
    }
  }, [
    hasNextPage,
    isFetchingNextPage,
    fetchNextPage,
    filteredConversationEntries.length,
    rowVirtualizer,
  ]);

  useEffect(() => {
    let cancelled = false;
    const query = debouncedRecipientQuery;
    if (query.length < 2) {
      setSearchResults([]);
      setSearchError(null);
      setIsSearching(false);
      return;
    }
    setIsSearching(true);
    (async () => {
      try {
        const response = await TauriApi.searchUsers({
          query,
          limit: 8,
          allowIncomplete: true,
        });
        if (cancelled) {
          return;
        }
        const mapped = response.items.map(mapUserProfileToUser);
        setSearchResults(mapped);
        setSearchError(null);
        errorHandler.info(
          'DirectMessageInbox.search_completed',
          'DirectMessageInbox.recipientSearch',
          {
            queryLength: query.length,
            resultCount: mapped.length,
          },
        );
      } catch (error) {
        if (cancelled) {
          return;
        }
        errorHandler.log('DirectMessageInbox.search_failed', error, {
          context: 'DirectMessageInbox.recipientSearch',
          metadata: { query },
        });
        setSearchError(t('dm.searchFailed'));
      } finally {
        if (!cancelled) {
          setIsSearching(false);
        }
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [debouncedRecipientQuery]);

  const handleClose = () => {
    closeInbox();
    setValidationError(null);
    setTargetNpub('');
    setSearchResults([]);
    setSearchError(null);
    setIsSearching(false);
    setConversationQuery('');
  };

  const handleStartConversation = () => {
    const npub = targetNpub.trim();
    if (!npub) {
      setValidationError(t('dm.enterTarget'));
      return;
    }
    if (currentUser?.npub === npub) {
      setValidationError(t('dm.cannotMessageSelf'));
      return;
    }
    setValidationError(null);
    closeInbox();
    setTargetNpub('');
    try {
      openDialog(npub);
    } catch (error) {
      errorHandler.log('DirectMessageInbox.open_failed', error, {
        context: 'DirectMessageInbox.handleStartConversation',
        metadata: { npub },
      });
    }
  };

  const handleSuggestionClick = (profile: Profile) => {
    const candidate = profile.npub || profile.id;
    if (!candidate || currentUser?.npub === candidate) {
      return;
    }
    setValidationError(null);
    closeInbox();
    setTargetNpub('');
    setSearchResults([]);
    try {
      openDialog(candidate);
    } catch (error) {
      errorHandler.log('DirectMessageInbox.open_failed', error, {
        context: 'DirectMessageInbox.handleSuggestionClick',
        metadata: { npub: candidate },
      });
    }
  };

  const handleMarkConversationRead = (npub: string, lastMessageAt: number | null) => {
    markConversationAsRead(npub, lastMessageAt ?? undefined);
    if (!lastMessageAt) {
      return;
    }
    void (async () => {
      try {
        await TauriApi.markDirectMessageConversationRead({
          conversationNpub: npub,
          lastReadAt: lastMessageAt,
        });
      } catch (error) {
        errorHandler.log('DirectMessageInbox.mark_read_failed', error, {
          context: 'DirectMessageInbox.handleMarkConversationRead',
          metadata: { npub },
        });
      }
    })();
  };

  const handleOpenConversation = (npub: string) => {
    closeInbox();
    openDialog(npub);
  };

  const handleConversationKeyDown = (event: KeyboardEvent<HTMLDivElement>, npub: string) => {
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      handleOpenConversation(npub);
    }
  };

  const latestConversationNpub = conversationEntries[0]?.npub ?? null;
  const handleConversationSearchKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === 'Enter' && autoCompleteConversationNpub) {
      event.preventDefault();
      handleOpenConversation(autoCompleteConversationNpub);
    } else if (event.key === 'Escape') {
      setConversationQuery('');
    }
  };

  return (
    <Dialog open={isInboxOpen} onOpenChange={(open) => (!open ? handleClose() : undefined)}>
      {isInboxOpen ? (
        <span className="sr-only" aria-live="polite">
          {t('dm.directMessages')}
        </span>
      ) : null}
      <DialogContent className="max-w-lg space-y-4">
        <DialogHeader>
          <DialogTitle>{t('dm.inboxTitle')}</DialogTitle>
          <p className="text-sm text-muted-foreground">{t('dm.inboxDescription')}</p>
        </DialogHeader>

        <div className="rounded-md border border-border p-4 space-y-3">
          <div className="flex items-center gap-2">
            <Input
              placeholder={t('dm.targetPlaceholder')}
              value={targetNpub}
              onChange={(event) => setTargetNpub(event.target.value)}
              data-testid="dm-inbox-target-input"
            />
            <Button onClick={handleStartConversation} data-testid="dm-inbox-start-button">
              {t('dm.newMessage')}
            </Button>
          </div>
          {validationError ? (
            <p className="text-xs text-destructive" data-testid="dm-inbox-error">
              {validationError}
            </p>
          ) : (
            <p className="text-xs text-muted-foreground">{t('dm.targetHint')}</p>
          )}
        </div>

        {debouncedRecipientQuery.length >= 2 && (
          <div className="rounded-md border border-dashed border-border/70 p-3 space-y-2">
            <div className="flex items-center gap-2 text-xs font-medium uppercase tracking-wide text-muted-foreground">
              <SearchIcon className="h-3.5 w-3.5" />
              {t('dm.candidates')}
              {isSearching && <Loader2 className="h-3 w-3 animate-spin text-muted-foreground" />}
            </div>
            {searchError ? (
              <p className="text-xs text-destructive">{searchError}</p>
            ) : searchResults.length === 0 ? (
              <p className="text-xs text-muted-foreground">{t('dm.noCandidates')}</p>
            ) : (
              <ul className="space-y-1" data-testid="dm-inbox-suggestions">
                {searchResults.slice(0, 5).map((profile) => {
                  const key = profile.npub || profile.id || profile.displayName || 'candidate';
                  const displayName = profile.displayName || profile.name || key;
                  return (
                    <li key={key}>
                      <button
                        type="button"
                        className="w-full rounded-md border border-border/70 px-3 py-2 text-left hover:bg-muted transition-colors flex items-center gap-3"
                        onClick={() => handleSuggestionClick(profile)}
                        data-testid={`dm-inbox-suggestion-${key}`}
                      >
                        <Avatar className="h-8 w-8">
                          <AvatarImage src={profile.picture || undefined} />
                          <AvatarFallback>{(displayName[0] ?? 'U').toUpperCase()}</AvatarFallback>
                        </Avatar>
                        <div className="overflow-hidden">
                          <p className="text-sm font-medium truncate">{displayName}</p>
                          <p className="text-xs text-muted-foreground truncate">
                            {profile.nip05 || profile.npub || profile.id}
                          </p>
                        </div>
                      </button>
                    </li>
                  );
                })}
              </ul>
            )}
          </div>
        )}

        <div className="space-y-3">
          <div className="flex items-center justify-between">
            <h2 className="text-sm font-medium text-muted-foreground">
              {t('dm.recentConversations')}
            </h2>
            <Button
              size="sm"
              variant="outline"
              onClick={() =>
                latestConversationNpub && handleOpenConversation(latestConversationNpub)
              }
              disabled={!latestConversationNpub}
              data-testid="dm-inbox-open-latest"
            >
              {t('dm.openLatest')}
            </Button>
          </div>
          <div className="flex items-center gap-2">
            <Input
              placeholder={t('dm.searchConversations')}
              value={conversationQuery}
              onChange={(event) => setConversationQuery(event.target.value)}
              onKeyDown={handleConversationSearchKeyDown}
              aria-label={t('dm.searchConversationsLabel')}
              data-testid="dm-inbox-conversation-search"
            />
            {conversationQuery ? (
              <Button
                type="button"
                size="sm"
                variant="ghost"
                onClick={() => setConversationQuery('')}
                data-testid="dm-inbox-clear-search"
              >
                {t('dm.clear')}
              </Button>
            ) : null}
          </div>
          <div className="flex items-center justify-between text-[11px] text-muted-foreground">
            <span>
              {t('dm.showing', {
                filtered: filteredConversationEntries.length,
                total: conversationEntries.length,
              })}
            </span>
            <div className="flex items-center gap-2">
              {conversationQuery && autoCompleteConversationNpub ? (
                <span>
                  {t('dm.openWithEnter', { npub: formatNpub(autoCompleteConversationNpub) })}
                </span>
              ) : null}
              {hasNextPage ? (
                <Button
                  type="button"
                  size="sm"
                  variant="ghost"
                  onClick={() => fetchNextPage()}
                  disabled={isFetchingNextPage}
                  data-testid="dm-inbox-load-more"
                >
                  {isFetchingNextPage ? t('dm.loadingMore') : t('dm.loadMore')}
                </Button>
              ) : null}
            </div>
          </div>
          {isConversationsError && hasConversations ? (
            <div className="text-[11px] text-destructive/80">{t('dm.fetchFailed')}</div>
          ) : null}
          <div
            ref={conversationListRef}
            className="h-60 rounded-md border border-border overflow-y-auto"
            data-testid="dm-inbox-list"
          >
            {showLoadingState ? (
              <div className="p-4 text-sm text-muted-foreground flex items-center gap-2">
                <Loader2 className="h-4 w-4 animate-spin" />
                <span>{t('dm.loadingConversations')}</span>
              </div>
            ) : showErrorState ? (
              <div className="p-4 space-y-2">
                <p className="text-sm text-destructive">{t('dm.fetchConversationsFailed')}</p>
                <Button size="sm" variant="outline" onClick={() => refetchConversations()}>
                  {t('dm.retry')}
                </Button>
                {conversationsError ? (
                  <p className="text-xs text-muted-foreground break-all">
                    {conversationsError.message}
                  </p>
                ) : null}
              </div>
            ) : !hasConversations ? (
              <div className="p-4 text-sm text-muted-foreground">{t('dm.noConversations')}</div>
            ) : !hasFilteredConversations ? (
              <div className="p-4 text-sm text-muted-foreground" data-testid="dm-inbox-no-results">
                {t('dm.noMatchingConversations', { query: conversationQuery })}
              </div>
            ) : shouldRenderFallbackList ? (
              <div className="divide-y divide-border/40">
                {filteredConversationEntries.map((entry) => {
                  const lastMessageTime = formatRelativeTime(entry.lastMessage?.createdAt);
                  const lastReadTime = entry.lastReadAt
                    ? formatRelativeTime(entry.lastReadAt)
                    : { display: null, helper: null };
                  const isSyncedRead =
                    entry.unread === 0 &&
                    entry.lastReadAt > 0 &&
                    (entry.lastMessage?.createdAt ?? 0) <= entry.lastReadAt;
                  const isHighlighted =
                    normalizedConversationQuery.length > 0 &&
                    autoCompleteConversationNpub === entry.npub;
                  return (
                    <div
                      key={entry.npub}
                      className={cn(
                        'w-full px-4 py-3 text-left hover:bg-muted transition-colors border-b border-border/40 last:border-b-0 rounded-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/30',
                        {
                          'ring-1 ring-primary/30 bg-primary/5': isHighlighted,
                        },
                      )}
                      role="button"
                      tabIndex={0}
                      aria-current={activeConversationNpub === entry.npub ? 'true' : undefined}
                      onClick={() => handleOpenConversation(entry.npub)}
                      onKeyDown={(event) => handleConversationKeyDown(event, entry.npub)}
                      data-testid={`dm-inbox-conversation-${entry.npub}`}
                    >
                      <div className="flex items-center justify-between gap-2">
                        <div className="min-w-0">
                          <p className="text-sm font-semibold">{formatNpub(entry.npub)}</p>
                          <p className="text-xs text-muted-foreground break-all">{entry.npub}</p>
                        </div>
                        <div className="flex items-center gap-2">
                          {isSyncedRead ? (
                            <Badge
                              variant="outline"
                              className="text-[11px]"
                              data-testid={`dm-inbox-read-sync-${entry.npub}`}
                            >
                              {t('dm.readSynced')}
                            </Badge>
                          ) : null}
                          {entry.unread > 0 ? (
                            <>
                              <Badge
                                variant="destructive"
                                data-testid={`dm-inbox-unread-${entry.npub}`}
                              >
                                {entry.unread > 99 ? '99+' : entry.unread}
                              </Badge>
                              <Button
                                type="button"
                                variant="ghost"
                                size="sm"
                                className="h-7 px-2 text-xs"
                                onClick={(event) => {
                                  event.stopPropagation();
                                  handleMarkConversationRead(
                                    entry.npub,
                                    entry.lastMessage?.createdAt ?? null,
                                  );
                                }}
                                data-testid={`dm-inbox-mark-read-${entry.npub}`}
                              >
                                {t('dm.markAsRead')}
                              </Button>
                            </>
                          ) : null}
                        </div>
                      </div>
                      <p className="text-xs text-muted-foreground truncate">
                        {entry.lastMessage?.content ?? t('dm.noMessages')}
                      </p>
                      <div className="flex items-center justify-between text-[11px] text-muted-foreground mt-1">
                        <span>
                          {t('dm.lastReceived')}:{' '}
                          {lastMessageTime.display ?? lastMessageTime.helper ?? '---'}
                        </span>
                        {activeConversationNpub === entry.npub ? (
                          <span>{t('dm.opening')}</span>
                        ) : null}
                      </div>
                      {entry.lastReadAt > 0 ? (
                        <div
                          className="text-[11px] text-muted-foreground"
                          data-testid={`dm-inbox-read-receipt-${entry.npub}`}
                        >
                          {t('dm.readSync')}: {lastReadTime.display ?? lastReadTime.helper ?? '---'}
                        </div>
                      ) : null}
                    </div>
                  );
                })}
              </div>
            ) : (
              <div
                style={{
                  height: `${rowVirtualizer.getTotalSize()}px`,
                  position: 'relative',
                }}
              >
                {virtualItems.map((virtualRow) => {
                  const entry = filteredConversationEntries[virtualRow.index];
                  if (!entry) {
                    return (
                      <div
                        key={virtualRow.key}
                        className="absolute left-0 right-0 flex items-center justify-center text-xs text-muted-foreground"
                        style={{
                          transform: `translateY(${virtualRow.start}px)`,
                          height: `${virtualRow.size}px`,
                        }}
                      >
                        {isFetchingNextPage ? (
                          <div className="flex items-center gap-2">
                            <Loader2 className="h-4 w-4 animate-spin" />
                            <span>{t('dm.loadingMore')}</span>
                          </div>
                        ) : (
                          <span>{t('dm.moreConversations')}</span>
                        )}
                      </div>
                    );
                  }
                  const lastMessageTime = formatRelativeTime(entry.lastMessage?.createdAt);
                  const lastReadTime = entry.lastReadAt
                    ? formatRelativeTime(entry.lastReadAt)
                    : { display: null, helper: null };
                  const isSyncedRead =
                    entry.unread === 0 &&
                    entry.lastReadAt > 0 &&
                    (entry.lastMessage?.createdAt ?? 0) <= entry.lastReadAt;
                  const isHighlighted =
                    normalizedConversationQuery.length > 0 &&
                    autoCompleteConversationNpub === entry.npub;
                  return (
                    <div
                      key={virtualRow.key}
                      ref={rowVirtualizer.measureElement}
                      className={cn(
                        'w-full px-4 py-3 text-left hover:bg-muted transition-colors absolute left-0 right-0 border-b border-border/40 last:border-b-0 rounded-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/30',
                        {
                          'ring-1 ring-primary/30 bg-primary/5': isHighlighted,
                        },
                      )}
                      style={{
                        transform: `translateY(${virtualRow.start}px)`,
                        height: `${virtualRow.size}px`,
                      }}
                      role="button"
                      tabIndex={0}
                      aria-current={activeConversationNpub === entry.npub ? 'true' : undefined}
                      onClick={() => handleOpenConversation(entry.npub)}
                      onKeyDown={(event) => handleConversationKeyDown(event, entry.npub)}
                      data-testid={`dm-inbox-conversation-${entry.npub}`}
                    >
                      <div className="flex items-center justify-between gap-2">
                        <div className="min-w-0">
                          <p className="text-sm font-semibold">{formatNpub(entry.npub)}</p>
                          <p className="text-xs text-muted-foreground break-all">{entry.npub}</p>
                        </div>
                        <div className="flex items-center gap-2">
                          {isSyncedRead ? (
                            <Badge
                              variant="outline"
                              className="text-[11px]"
                              data-testid={`dm-inbox-read-sync-${entry.npub}`}
                            >
                              {t('dm.readSynced')}
                            </Badge>
                          ) : null}
                          {entry.unread > 0 ? (
                            <>
                              <Badge
                                variant="destructive"
                                data-testid={`dm-inbox-unread-${entry.npub}`}
                              >
                                {entry.unread > 99 ? '99+' : entry.unread}
                              </Badge>
                              <Button
                                type="button"
                                variant="ghost"
                                size="sm"
                                className="h-7 px-2 text-xs"
                                onClick={(event) => {
                                  event.stopPropagation();
                                  handleMarkConversationRead(
                                    entry.npub,
                                    entry.lastMessage?.createdAt ?? null,
                                  );
                                }}
                                data-testid={`dm-inbox-mark-read-${entry.npub}`}
                              >
                                {t('dm.markAsRead')}
                              </Button>
                            </>
                          ) : null}
                        </div>
                      </div>
                      <p className="text-xs text-muted-foreground truncate">
                        {entry.lastMessage?.content ?? t('dm.noMessages')}
                      </p>
                      <div className="flex items-center justify-between text-[11px] text-muted-foreground mt-1">
                        <span>
                          {t('dm.lastReceived')}:{' '}
                          {lastMessageTime.display ?? lastMessageTime.helper ?? '---'}
                        </span>
                        {activeConversationNpub === entry.npub ? (
                          <span>{t('dm.opening')}</span>
                        ) : null}
                      </div>
                      {entry.lastReadAt > 0 ? (
                        <div
                          className="text-[11px] text-muted-foreground"
                          data-testid={`dm-inbox-read-receipt-${entry.npub}`}
                        >
                          {t('dm.readSync')}: {lastReadTime.display ?? lastReadTime.helper ?? '---'}
                        </div>
                      ) : null}
                    </div>
                  );
                })}
              </div>
            )}
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
