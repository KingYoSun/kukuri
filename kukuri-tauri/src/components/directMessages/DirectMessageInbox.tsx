import { useEffect, useMemo, useRef, useState } from 'react';
import type { KeyboardEvent } from 'react';
import { useInfiniteQuery, type InfiniteData } from '@tanstack/react-query';
import { useVirtualizer } from '@tanstack/react-virtual';
import { formatDistanceToNow } from 'date-fns';
import { ja } from 'date-fns/locale';
import { Loader2, Search as SearchIcon } from 'lucide-react';

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
    display: formatDistanceToNow(date, { addSuffix: true, locale: ja }),
    helper: date.toLocaleString('ja-JP'),
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

export function DirectMessageInbox() {
  const currentUser = useAuthStore((state) => state.currentUser);
  const isInboxOpen = useDirectMessageStore((state) => state.isInboxOpen);
  const closeInbox = useDirectMessageStore((state) => state.closeInbox);
  const openDialog = useDirectMessageStore((state) => state.openDialog);
  const activeConversationNpub = useDirectMessageStore((state) => state.activeConversationNpub);
  const markConversationAsRead = useDirectMessageStore((state) => state.markConversationAsRead);
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

  const conversationPages = conversationsQuery.data?.pages ?? [];
  const conversationEntries = useMemo<ConversationEntry[]>(() => {
    return conversationPages
      .flatMap((page: DirectMessageConversationList) => page.items)
      .map((item: DirectMessageConversationSummary) => ({
        npub: item.conversationNpub,
        lastMessage: item.lastMessage,
        unread: item.unreadCount,
        lastReadAt: item.lastReadAt,
      }));
  }, [conversationPages]);
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
  const rowVirtualizer = useVirtualizer({
    count: hasNextPage
      ? filteredConversationEntries.length + 1
      : filteredConversationEntries.length,
    getScrollElement: () => conversationListRef.current,
    estimateSize: () => 88,
    overscan: 12,
    getItemKey: (index) => filteredConversationEntries[index]?.npub ?? `loader-${index}`,
  });
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
        setSearchError('ユーザー検索に失敗しました');
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
      setValidationError('宛先の npub または ID を入力してください。');
      return;
    }
    if (currentUser?.npub === npub) {
      setValidationError('自分自身にはメッセージを送信できません。');
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
          ダイレクトメッセージ
        </span>
      ) : null}
      <DialogContent className="max-w-lg space-y-4">
        <DialogHeader>
          <DialogTitle>ダイレクトメッセージ</DialogTitle>
          <p className="text-sm text-muted-foreground">
            既存の会話を開くか、宛先を指定して新しいメッセージを開始できます。
          </p>
        </DialogHeader>

        <div className="rounded-md border border-border p-4 space-y-3">
          <div className="flex items-center gap-2">
            <Input
              placeholder="npub1... / ユーザーID"
              value={targetNpub}
              onChange={(event) => setTargetNpub(event.target.value)}
              data-testid="dm-inbox-target-input"
            />
            <Button onClick={handleStartConversation} data-testid="dm-inbox-start-button">
              新しいメッセージ
            </Button>
          </div>
          {validationError ? (
            <p className="text-xs text-destructive" data-testid="dm-inbox-error">
              {validationError}
            </p>
          ) : (
            <p className="text-xs text-muted-foreground">
              npub / ユーザーID を入力し、「新しいメッセージ」を押すとモーダルが開きます。
            </p>
          )}
        </div>

        {debouncedRecipientQuery.length >= 2 && (
          <div className="rounded-md border border-dashed border-border/70 p-3 space-y-2">
            <div className="flex items-center gap-2 text-xs font-medium uppercase tracking-wide text-muted-foreground">
              <SearchIcon className="h-3.5 w-3.5" />
              候補
              {isSearching && <Loader2 className="h-3 w-3 animate-spin text-muted-foreground" />}
            </div>
            {searchError ? (
              <p className="text-xs text-destructive">{searchError}</p>
            ) : searchResults.length === 0 ? (
              <p className="text-xs text-muted-foreground">一致する候補が見つかりません</p>
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
            <h2 className="text-sm font-medium text-muted-foreground">最近の会話</h2>
            <Button
              size="sm"
              variant="outline"
              onClick={() =>
                latestConversationNpub && handleOpenConversation(latestConversationNpub)
              }
              disabled={!latestConversationNpub}
              data-testid="dm-inbox-open-latest"
            >
              最新の会話を開く
            </Button>
          </div>
          <div className="flex items-center gap-2">
            <Input
              placeholder="会話を検索（npub / メッセージ本文）"
              value={conversationQuery}
              onChange={(event) => setConversationQuery(event.target.value)}
              onKeyDown={handleConversationSearchKeyDown}
              aria-label="会話検索"
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
                クリア
              </Button>
            ) : null}
          </div>
          <div className="flex items-center justify-between text-[11px] text-muted-foreground">
            <span>
              {filteredConversationEntries.length} 件 / 読み込み済み {conversationEntries.length} 件
            </span>
            <div className="flex items-center gap-2">
              {conversationQuery && autoCompleteConversationNpub ? (
                <span>Enter で {formatNpub(autoCompleteConversationNpub)} を開く</span>
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
                  {isFetchingNextPage ? '読み込み中…' : 'さらに表示'}
                </Button>
              ) : null}
            </div>
          </div>
          <div
            ref={conversationListRef}
            className="h-60 rounded-md border border-border overflow-y-auto"
            data-testid="dm-inbox-list"
          >
            {isConversationsLoading ? (
              <div className="p-4 text-sm text-muted-foreground flex items-center gap-2">
                <Loader2 className="h-4 w-4 animate-spin" />
                <span>会話を読み込んでいます…</span>
              </div>
            ) : isConversationsError ? (
              <div className="p-4 space-y-2">
                <p className="text-sm text-destructive">
                  会話の取得に失敗しました。時間をおいて再試行してください。
                </p>
                <Button size="sm" variant="outline" onClick={() => refetchConversations()}>
                  再試行
                </Button>
                {conversationsError ? (
                  <p className="text-xs text-muted-foreground break-all">
                    {conversationsError.message}
                  </p>
                ) : null}
              </div>
            ) : !hasConversations ? (
              <div className="p-4 text-sm text-muted-foreground">
                まだ会話がありません。プロフィールから、または上の宛先入力から開始できます。
              </div>
            ) : !hasFilteredConversations ? (
              <div className="p-4 text-sm text-muted-foreground" data-testid="dm-inbox-no-results">
                “{conversationQuery}” に一致する会話が見つかりません。
              </div>
            ) : (
              <div
                style={{
                  height: `${rowVirtualizer.getTotalSize()}px`,
                  position: 'relative',
                }}
              >
                {rowVirtualizer.getVirtualItems().map((virtualRow) => {
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
                            <span>さらに読み込み中…</span>
                          </div>
                        ) : (
                          <span>これ以上の会話はありません</span>
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
                              既読同期済
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
                                既読にする
                              </Button>
                            </>
                          ) : null}
                        </div>
                      </div>
                      <p className="text-xs text-muted-foreground truncate">
                        {entry.lastMessage?.content ?? 'メッセージはまだありません'}
                      </p>
                      <div className="flex items-center justify-between text-[11px] text-muted-foreground mt-1">
                        <span>
                          最終受信: {lastMessageTime.display ?? lastMessageTime.helper ?? '---'}
                        </span>
                        {activeConversationNpub === entry.npub ? <span>開いています</span> : null}
                      </div>
                      {entry.lastReadAt > 0 ? (
                        <div
                          className="text-[11px] text-muted-foreground"
                          data-testid={`dm-inbox-read-receipt-${entry.npub}`}
                        >
                          既読同期: {lastReadTime.display ?? lastReadTime.helper ?? '---'}
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
