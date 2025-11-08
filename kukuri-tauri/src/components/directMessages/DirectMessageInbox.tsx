import { useMemo, useState } from 'react';
import { formatDistanceToNow } from 'date-fns';
import { ja } from 'date-fns/locale';

import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Badge } from '@/components/ui/badge';
import { useDirectMessageStore } from '@/stores/directMessageStore';
import { useAuthStore } from '@/stores/authStore';
import { errorHandler } from '@/lib/errorHandler';

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
  } = useDirectMessageStore((state) => ({
    isInboxOpen: state.isInboxOpen,
    closeInbox: state.closeInbox,
    openDialog: state.openDialog,
    conversations: state.conversations,
    unreadCounts: state.unreadCounts,
    activeConversationNpub: state.activeConversationNpub,
  }));
  const [targetNpub, setTargetNpub] = useState('');
  const [validationError, setValidationError] = useState<string | null>(null);

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

  const handleClose = () => {
    closeInbox();
    setValidationError(null);
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

  const handleOpenConversation = (npub: string) => {
    closeInbox();
    openDialog(npub);
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
          <ScrollArea className="h-60 rounded-md border border-border">
            {conversationEntries.length === 0 ? (
              <div className="p-4 text-sm text-muted-foreground">
                まだ会話がありません。プロフィールから、または上の宛先入力から開始できます。
              </div>
            ) : (
              <div className="divide-y">
                {conversationEntries.map(({ npub, lastMessage, unread }) => {
                  const { display } = formatRelativeTime(lastMessage?.createdAt);
                  return (
                    <button
                      key={npub}
                      type="button"
                      className="w-full px-4 py-3 text-left hover:bg-muted transition-colors"
                      onClick={() => handleOpenConversation(npub)}
                      data-testid={`dm-inbox-conversation-${npub}`}
                    >
                      <div className="flex items-center justify-between">
                        <p className="text-sm font-medium break-all">{npub}</p>
                        {unread > 0 ? (
                          <Badge variant="destructive" data-testid={`dm-inbox-unread-${npub}`}>
                            {unread > 99 ? '99+' : unread}
                          </Badge>
                        ) : null}
                      </div>
                      <p className="text-xs text-muted-foreground truncate">
                        {lastMessage?.content ?? 'メッセージはまだありません'}
                      </p>
                      <div className="flex items-center justify-between text-[11px] text-muted-foreground mt-1">
                        <span>{display ?? '未受信'}</span>
                        {activeConversationNpub === npub ? <span>開いています</span> : null}
                      </div>
                    </button>
                  );
                })}
              </div>
            )}
          </ScrollArea>
        </div>
      </DialogContent>
    </Dialog>
  );
}
