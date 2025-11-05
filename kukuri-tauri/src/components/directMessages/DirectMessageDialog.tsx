import { useMemo, useCallback } from 'react';
import { toast } from 'sonner';
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';
import { ScrollArea } from '@/components/ui/scroll-area';
import { cn } from '@/lib/utils';
import { errorHandler } from '@/lib/errorHandler';
import { TauriApi } from '@/lib/api/tauri';
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
  }));

  const messages = useMemo(() => {
    if (!activeConversationNpub) {
      return [] as DirectMessageModel[];
    }
    const confirmed = conversations[activeConversationNpub] ?? [];
    const pending = optimisticMessages[activeConversationNpub] ?? [];
    return [...confirmed, ...pending].sort((a, b) => a.createdAt - b.createdAt);
  }, [activeConversationNpub, conversations, optimisticMessages]);

  const handleSend = useCallback(async () => {
    if (!activeConversationNpub || !currentUser) {
      toast.error('メッセージを送信するにはログインが必要です。');
      return;
    }
    const trimmed = messageDraft.trim();
    if (!trimmed) {
      return;
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
    setDraft('');
    setIsSending(true);

    try {
      const response = await TauriApi.sendDirectMessage({
        recipientNpub: activeConversationNpub,
        content: trimmed,
        clientMessageId,
      });
      resolveOptimisticMessage(activeConversationNpub, clientMessageId, response?.eventId ?? null);
      toast.success('メッセージを送信しました。');
    } catch (error) {
      failOptimisticMessage(activeConversationNpub, clientMessageId, error);
      toast.error('メッセージの送信に失敗しました。');
      errorHandler.log('DirectMessageDialog.sendFailed', error, {
        context: 'DirectMessageDialog.handleSend',
        metadata: { recipient: activeConversationNpub, clientMessageId },
      });
    } finally {
      setIsSending(false);
    }
  }, [
    activeConversationNpub,
    appendOptimisticMessage,
    currentUser,
    failOptimisticMessage,
    messageDraft,
    resolveOptimisticMessage,
    setDraft,
    setIsSending,
  ]);

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
        <ScrollArea className="h-72 rounded-md border border-border p-3">
          <div className="flex flex-col gap-3">
            {messages.length === 0 ? (
              <p className="text-sm text-muted-foreground">
                まだメッセージはありません。最初のメッセージを送信しましょう。
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
                        ? ' · 送信中…'
                        : message.status === 'failed'
                          ? ' · 送信失敗'
                          : ''}
                    </span>
                  </div>
                );
              })
            )}
          </div>
        </ScrollArea>
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
