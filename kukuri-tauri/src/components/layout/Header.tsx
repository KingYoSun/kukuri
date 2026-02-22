import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import { Bell, Menu, MessageCircle, Plus } from 'lucide-react';
import { useUIStore, useTopicStore } from '@/stores';
import { useNavigate } from '@tanstack/react-router';
import { AccountSwitcher } from '@/components/auth/AccountSwitcher';
import { RealtimeIndicator } from '@/components/RealtimeIndicator';
import { SyncStatusIndicator } from '@/components/SyncStatusIndicator';
import { useDirectMessageStore } from '@/stores/directMessageStore';
import { useDirectMessageBadge } from '@/hooks/useDirectMessageBadge';
import { DirectMessageInbox } from '@/components/directMessages/DirectMessageInbox';
import { useDirectMessageBootstrap } from '@/hooks/useDirectMessageBootstrap';
import { AntennaStatusDialog } from '@/components/layout/AntennaStatusDialog';

export function Header() {
  const { t } = useTranslation();
  const { toggleSidebar } = useUIStore();
  const { setCurrentTopic } = useTopicStore();
  const navigate = useNavigate();
  const { unreadTotal, latestConversationNpub } = useDirectMessageBadge();
  const openDialog = useDirectMessageStore((state) => state.openDialog);
  const openInbox = useDirectMessageStore((state) => state.openInbox);
  const activeConversationNpub = useDirectMessageStore((state) => state.activeConversationNpub);

  useDirectMessageBootstrap();

  return (
    <>
      <header
        role="banner"
        className="h-16 border-b bg-background px-6 flex items-center justify-between"
      >
        <div className="flex items-center gap-4">
          <Button
            variant="ghost"
            size="icon"
            onClick={toggleSidebar}
            aria-label={t('nav.toggleMenu')}
          >
            <Menu className="h-5 w-5" />
          </Button>
          <h1
            className="text-2xl font-bold cursor-pointer hover:opacity-80 transition-opacity"
            onClick={() => {
              setCurrentTopic(null); // 全体のタイムラインを表示
              navigate({ to: '/' });
            }}
          >
            kukuri
          </h1>
        </div>

        <div className="flex items-center gap-4">
          <RealtimeIndicator />
          <SyncStatusIndicator />
          <AntennaStatusDialog />

          <Button
            variant="ghost"
            size="icon"
            className="relative"
            aria-label={t('nav.directMessage')}
            onClick={() => {
              const target = activeConversationNpub ?? latestConversationNpub;
              if (target) {
                openDialog(target);
                return;
              }
              openInbox();
            }}
          >
            <MessageCircle className="h-5 w-5" />
            {unreadTotal > 0 && (
              <span className="absolute -top-1 -right-1 rounded-full bg-destructive px-1.5 text-[0.625rem] font-semibold text-destructive-foreground">
                {unreadTotal > 99 ? '99+' : unreadTotal}
              </span>
            )}
          </Button>
          <Button
            variant="outline"
            size="icon"
            aria-label={t('nav.newDirectMessage')}
            onClick={openInbox}
            data-testid="open-dm-inbox-button"
          >
            <Plus className="h-5 w-5" />
          </Button>

          <Button variant="ghost" size="icon" aria-label={t('nav.notifications')}>
            <Bell className="h-5 w-5" />
          </Button>

          <AccountSwitcher />
        </div>
      </header>
      <DirectMessageInbox />
    </>
  );
}
