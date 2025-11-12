import { useEffect, useMemo, useRef, useState } from 'react';
import type { KeyboardEvent } from 'react';
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
import { TauriApi } from '@/lib/api/tauri';

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

export function DirectMessageInbox() {
  const currentUser = useAuthStore((state) => state.currentUser);
  const {
    isInboxOpen,
    closeInbox,
    openDialog,
    conversations,
    unreadCounts,
    activeConversationNpub,
    markConversationAsRead,
  } = useDirectMessageStore((state) => ({
    isInboxOpen: state.isInboxOpen,
    closeInbox: state.closeInbox,
    openDialog: state.openDialog,
    conversations: state.conversations,
    unreadCounts: state.unreadCounts,
    activeConversationNpub: state.activeConversationNpub,
    markConversationAsRead: state.markConversationAsRead,
  }));
  const [targetNpub, setTargetNpub] = useState('');
  const [validationError, setValidationError] = useState<string | null>(null);
  const [searchResults, setSearchResults] = useState<Profile[]>([]);
  const [isSearching, setIsSearching] = useState(false);
  const [searchError, setSearchError] = useState<string | null>(null);
  const debouncedRecipientQuery = useDebounce(targetNpub.trim(), 300);

  const conversationEntries = useMemo(() => {
    return Object.entries(conversations)
      .map(([npub, messages]) => {
        const lastMessage = messages[messages.length - 1] ?? null;
        return {
          npub,
          lastMessage,
          unread: unreadCounts[npub] ?? 0,
        };
      })
      .sort((a, b) => {
        const aTime = a.lastMessage?.createdAt ?? 0;
        const bTime = b.lastMessage?.createdAt ?? 0;
        return bTime - aTime;
      });
  }, [conversations, unreadCounts]);
  const hasConversations = conversationEntries.length > 0;
  const conversationListRef = useRef<HTMLDivElement | null>(null);
  const rowVirtualizer = useVirtualizer({
    count: conversationEntries.length,
    getScrollElement: () => conversationListRef.current,
    estimateSize: () => 76,
    overscan: 8,
  });

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
        errorHandler.info('DirectMessageInbox.search_completed', 'DirectMessageInbox.recipientSearch', {
          queryLength: query.length,
          resultCount: mapped.length,
        });
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
    setSearchResults([]);
    setSearchError(null);
    setIsSearching(false);
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
    markConversationAsRead(npub);
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

        <div className="space-y-2">
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
          <div
            ref={conversationListRef}
            className="h-60 rounded-md border border-border overflow-y-auto"
            data-testid="dm-inbox-list"
          >
            {!hasConversations ? (
              <div className="p-4 text-sm text-muted-foreground">
                まだ会話がありません。プロフィールから、または上の宛先入力から開始できます。
              </div>
            ) : (
              <div
                style={{
                  height: `${rowVirtualizer.getTotalSize()}px`,
                  position: 'relative',
                }}
              >
                {rowVirtualizer.getVirtualItems().map((virtualRow) => {
                  const entry = conversationEntries[virtualRow.index];
                  const { display } = formatRelativeTime(entry.lastMessage?.createdAt);
                  return (
                    <div
                      key={entry.npub}
                      className="w-full px-4 py-3 text-left hover:bg-muted transition-colors absolute left-0 right-0 border-b border-border/40 last:border-b-0"
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
                        <p className="text-sm font-medium break-all">{entry.npub}</p>
                        <div className="flex items-center gap-2">
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
                        <span>{display ?? '未受信'}</span>
                        {activeConversationNpub === entry.npub ? <span>開いています</span> : null}
                      </div>
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
