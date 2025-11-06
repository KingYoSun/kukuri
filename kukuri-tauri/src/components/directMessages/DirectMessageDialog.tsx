import { useMemo, useCallback, useEffect, useRef } from 'react';
import { useInfiniteQuery, type InfiniteData } from '@tanstack/react-query';
import { Loader2 } from 'lucide-react';
import { toast } from 'sonner';
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';
import { ScrollArea } from '@/components/ui/scroll-area';
import { cn } from '@/lib/utils';
import { errorHandler } from '@/lib/errorHandler';
import { TauriApi, type DirectMessageItem, type DirectMessagePage } from '@/lib/api/tauri';
import { useAuthStore } from '@/stores/authStore';
import { useDirectMessageStore, type DirectMessageModel } from '@/stores/directMessageStore';

const fallbackMessageId = () => `dm_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;

const formatTimestamp = (timestamp: number) => {
  try {
    return new Intl.DateTimeFormat('ja-JP', {
      hour: '2-digit',
      minute: '2-digit',
    }).format(new Date(timestamp));
  } catch {
    return '';
  }
};

const mapApiMessageToModel = (item: DirectMessageItem): DirectMessageModel => ({
  eventId: item.eventId,
  clientMessageId:
    item.clientMessageId ?? item.eventId ?? `generated-${item.senderNpub}-${item.createdAt}`,
  senderNpub: item.senderNpub,
  recipientNpub: item.recipientNpub,
  content: item.content,
  createdAt: item.createdAt,
  status: item.delivered ? 'sent' : 'pending',
});

export function DirectMessageDialog() {
  const currentUser = useAuthStore((state) => state.currentUser);
  const {
    isDialogOpen,
    activeConversationNpub,
    messageDraft,
    isSending,
    conversations,
    optimisticMessages,
    closeDialog,
    setDraft,
    setIsSending,
    appendOptimisticMessage,
    resolveOptimisticMessage,
    failOptimisticMessage,
    setMessages,
    markConversationAsRead,
    removeOptimisticMessage,
  } = useDirectMessageStore((state) => ({
    isDialogOpen: state.isDialogOpen,
    activeConversationNpub: state.activeConversationNpub,
    messageDraft: state.messageDraft,
    isSending: state.isSending,
    conversations: state.conversations,
    optimisticMessages: state.optimisticMessages,
    closeDialog: state.closeDialog,
    setDraft: state.setDraft,
    setIsSending: state.setIsSending,
    appendOptimisticMessage: state.appendOptimisticMessage,
    resolveOptimisticMessage: state.resolveOptimisticMessage,
    failOptimisticMessage: state.failOptimisticMessage,
    setMessages: state.setMessages,
    markConversationAsRead: state.markConversationAsRead,
    removeOptimisticMessage: state.removeOptimisticMessage,
  }));

  const scrollAreaWrapperRef = useRef<HTMLDivElement | null>(null);
  const scrollViewportRef = useRef<HTMLDivElement | null>(null);
  const topSentinelRef = useRef<HTMLDivElement | null>(null);
  const autoLoadLockRef = useRef(false);

  useEffect(() => {
    if (!isDialogOpen) {
      scrollViewportRef.current = null;
      return;
    }
    const wrapper = scrollAreaWrapperRef.current;
    if (!wrapper) {
      return;
    }
    const viewport = wrapper.querySelector(
      '[data-slot="scroll-area-viewport"]',
    ) as HTMLDivElement | null;
    scrollViewportRef.current = viewport ?? null;
  }, [isDialogOpen, activeConversationNpub]);

  useEffect(() => {
    autoLoadLockRef.current = false;
  }, [activeConversationNpub, isDialogOpen]);

  useEffect(() => {
    const viewport = scrollViewportRef.current;
    if (!viewport || !isDialogOpen) {
      return;
    }
    const handleScroll = () => {
      if (viewport.scrollTop > 48) {
        autoLoadLockRef.current = false;
      }
    };
    viewport.addEventListener('scroll', handleScroll);
    return () => {
      viewport.removeEventListener('scroll', handleScroll);
    };
  }, [isDialogOpen, activeConversationNpub]);

  const directMessagesQuery = useInfiniteQuery<
    DirectMessagePage,
    Error,
    InfiniteData<DirectMessagePage, string | null>,
    ['direct-messages', string],
    string | null
  >({
    queryKey: ['direct-messages', activeConversationNpub ?? 'inactive'],
    enabled: isDialogOpen && Boolean(activeConversationNpub),
    retry: false,
    initialPageParam: null,
    queryFn: async ({ pageParam }) => {
      if (!activeConversationNpub) {
        return { items: [], nextCursor: null, hasMore: false };
      }
      return await TauriApi.listDirectMessages({
        conversationNpub: activeConversationNpub,
        cursor: pageParam ?? null,
        limit: 30,
        direction: 'backward',
      });
    },
    getNextPageParam: (lastPage) =>
      lastPage.hasMore ? (lastPage.nextCursor ?? undefined) : undefined,
  });

  const {
    data: directMessagePages,
    isLoading: isHistoryLoading,
    isError: isHistoryError,
    error: historyError,
    fetchNextPage,
    hasNextPage,
    isFetchingNextPage,
    refetch: refetchHistory,
    isSuccess: isHistorySuccess,
  } = directMessagesQuery;

  useEffect(() => {
    if (!isDialogOpen) {
      return;
    }
    const sentinel = topSentinelRef.current;
    const viewport = scrollViewportRef.current;
    if (!sentinel || !viewport) {
      return;
    }
    if (!directMessagePages || directMessagePages.pages.length === 0) {
      return;
    }
    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (
            entry.isIntersecting &&
            hasNextPage &&
            !isFetchingNextPage &&
            viewport.scrollTop <= 48 &&
            !autoLoadLockRef.current
          ) {
            autoLoadLockRef.current = true;
            void fetchNextPage();
          }
        }
      },
      { root: viewport, threshold: 0.1 },
    );
    observer.observe(sentinel);
    return () => {
      observer.disconnect();
    };
  }, [isDialogOpen, directMessagePages, fetchNextPage, hasNextPage, isFetchingNextPage]);

  const confirmedFromQuery = useMemo(() => {
    if (!isHistorySuccess || !directMessagePages) {
      return undefined;
    }
    return directMessagePages.pages
      .flatMap((page) => page.items)
      .map(mapApiMessageToModel)
      .sort((a, b) => a.createdAt - b.createdAt);
  }, [directMessagePages, isHistorySuccess]);

  useEffect(() => {
    if (!activeConversationNpub || confirmedFromQuery === undefined) {
      return;
    }
    setMessages(activeConversationNpub, confirmedFromQuery, { replace: true });
    markConversationAsRead(activeConversationNpub);
  }, [activeConversationNpub, confirmedFromQuery, setMessages, markConversationAsRead]);

  useEffect(() => {
    if (!isHistoryError || !historyError || !activeConversationNpub) {
      return;
    }
    errorHandler.log('DirectMessageDialog.historyLoadFailed', historyError, {
      context: 'DirectMessageDialog.useInfiniteQuery',
      metadata: { conversation: activeConversationNpub },
    });
  }, [isHistoryError, historyError, activeConversationNpub]);

  const messages = useMemo(() => {
    if (!activeConversationNpub) {
      return [] as DirectMessageModel[];
    }
    const confirmed = confirmedFromQuery ?? conversations[activeConversationNpub] ?? [];
    const pending = optimisticMessages[activeConversationNpub] ?? [];
    return [...confirmed, ...pending].sort((a, b) => a.createdAt - b.createdAt);
  }, [activeConversationNpub, confirmedFromQuery, conversations, optimisticMessages]);

  const hasLoadedAtLeastOnePage = Boolean(
    directMessagePages && directMessagePages.pages.length > 0,
  );
  const initialLoading = isHistoryLoading && messages.length === 0;
  const showLoadMoreButton = hasLoadedAtLeastOnePage && Boolean(hasNextPage) && !isFetchingNextPage;
  const showTopSpinner = hasLoadedAtLeastOnePage && isFetchingNextPage;

  const sendMessage = useCallback(
    async (
      rawContent: string,
      options: { retryingClientId?: string; preserveDraft?: boolean } = {},
    ) => {
      if (!activeConversationNpub || !currentUser) {
        toast.error('メッセージを送信するにはログインが必要です。');
        return;
      }

      const trimmed = rawContent.trim();
      if (!trimmed || isSending) {
        return;
      }

      if (options.retryingClientId) {
        removeOptimisticMessage(activeConversationNpub, options.retryingClientId);
      }

      const clientMessageId =
        typeof crypto !== 'undefined' && 'randomUUID' in crypto
          ? crypto.randomUUID()
          : fallbackMessageId();

      const optimistic: DirectMessageModel = {
        eventId: null,
        clientMessageId,
        senderNpub: currentUser.npub,
        recipientNpub: activeConversationNpub,
        content: trimmed,
        createdAt: Date.now(),
        status: 'pending',
      };

      appendOptimisticMessage(activeConversationNpub, optimistic);
      if (!options.preserveDraft) {
        setDraft('');
      }
      setIsSending(true);

      try {
        const response = await TauriApi.sendDirectMessage({
          recipientNpub: activeConversationNpub,
          content: trimmed,
          clientMessageId,
        });
        resolveOptimisticMessage(
          activeConversationNpub,
          clientMessageId,
          response?.eventId ?? null,
        );
        toast.success('メッセージを送信しました。');
      } catch (error) {
        failOptimisticMessage(activeConversationNpub, clientMessageId, error);
        toast.error('メッセージの送信に失敗しました。');
        errorHandler.log('DirectMessageDialog.sendFailed', error, {
          context: options.retryingClientId
            ? 'DirectMessageDialog.retrySend'
            : 'DirectMessageDialog.handleSend',
          metadata: { recipient: activeConversationNpub, clientMessageId },
        });
      } finally {
        setIsSending(false);
      }
    },
    [
      activeConversationNpub,
      appendOptimisticMessage,
      currentUser,
      failOptimisticMessage,
      isSending,
      removeOptimisticMessage,
      resolveOptimisticMessage,
      setDraft,
      setIsSending,
    ],
  );

  const handleSend = useCallback(async () => {
    await sendMessage(messageDraft);
  }, [messageDraft, sendMessage]);

  const handleRetry = useCallback(
    async (target: DirectMessageModel) => {
      await sendMessage(target.content, {
        retryingClientId: target.clientMessageId,
        preserveDraft: true,
      });
    },
    [sendMessage],
  );

  if (!isDialogOpen || !activeConversationNpub) {
    return null;
  }

  const handleClose = () => {
    setDraft('');
    setIsSending(false);
    closeDialog();
  };

  const isSendDisabled = !messageDraft.trim() || isSending || !currentUser;

  return (
    <Dialog open={isDialogOpen} onOpenChange={(open) => !open && handleClose()}>
      <DialogContent className="max-w-xl space-y-4">
        <DialogHeader>
          <DialogTitle>ダイレクトメッセージ</DialogTitle>
          <p className="text-sm text-muted-foreground break-all">宛先: {activeConversationNpub}</p>
        </DialogHeader>
        <div ref={scrollAreaWrapperRef}>
          <ScrollArea className="h-72 rounded-md border border-border">
            <div className="flex flex-col gap-3 p-3">
              <div ref={topSentinelRef} className="h-1 w-full" />
              {showLoadMoreButton && (
                <div className="flex justify-center">
                  <Button
                    size="sm"
                    variant="outline"
                    onClick={() => {
                      autoLoadLockRef.current = true;
                      void fetchNextPage();
                    }}
                  >
                    過去のメッセージを読み込む
                  </Button>
                </div>
              )}
              {showTopSpinner && (
                <div className="flex items-center justify-center gap-2 text-xs text-muted-foreground">
                  <Loader2 className="h-3.5 w-3.5 animate-spin" />
                  <span>過去のメッセージを読み込み中…</span>
                </div>
              )}
              {isHistoryError && (
                <div className="flex items-center justify-between gap-3 rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-xs text-destructive">
                  <span>メッセージ履歴の取得に失敗しました。</span>
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={() => {
                      void refetchHistory();
                    }}
                  >
                    再試行
                  </Button>
                </div>
              )}
              {initialLoading ? (
                <div className="py-8 text-center text-sm text-muted-foreground">
                  メッセージ履歴を読み込み中…
                </div>
              ) : messages.length === 0 ? (
                <p className="text-sm text-muted-foreground">
                  まだメッセージはありません。最初のメッセージを送信してみましょう。
                </p>
              ) : (
                messages.map((message) => {
                  const isSelf = message.senderNpub === currentUser?.npub;
                  return (
                    <div
                      key={`${message.eventId ?? 'pending'}:${message.clientMessageId}`}
                      className={cn('flex flex-col gap-1 max-w-[80%]', {
                        'ml-auto items-end': isSelf,
                        'mr-auto items-start': !isSelf,
                      })}
                      data-testid="direct-message-item"
                    >
                      <div
                        className={cn('rounded-lg px-3 py-2 text-sm shadow-sm', {
                          'bg-primary text-primary-foreground': isSelf,
                          'bg-muted text-foreground': !isSelf,
                        })}
                      >
                        {message.content}
                      </div>
                      <span className="text-xs text-muted-foreground">
                        {formatTimestamp(message.createdAt)}
                        {message.status === 'pending'
                          ? ' ・送信中'
                          : message.status === 'failed'
                            ? ' ・送信失敗'
                            : ''}
                      </span>
                      {isSelf && message.status === 'failed' && (
                        <Button
                          size="sm"
                          variant="link"
                          className="h-auto px-0 text-xs"
                          onClick={() => {
                            void handleRetry(message);
                          }}
                        >
                          再送
                        </Button>
                      )}
                    </div>
                  );
                })
              )}
            </div>
          </ScrollArea>
        </div>
        <div className="space-y-3">
          <Textarea
            value={messageDraft}
            onChange={(event) => setDraft(event.target.value)}
            placeholder="メッセージを入力してください…"
            onKeyDown={(event) => {
              if (event.key === 'Enter' && (event.metaKey || event.ctrlKey)) {
                event.preventDefault();
                void handleSend();
              }
            }}
            minRows={3}
            data-testid="direct-message-input"
          />
          <div className="flex justify-end gap-2">
            <Button variant="outline" onClick={handleClose}>
              キャンセル
            </Button>
            <Button onClick={() => void handleSend()} disabled={isSendDisabled}>
              {isSending ? '送信中…' : '送信'}
            </Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
