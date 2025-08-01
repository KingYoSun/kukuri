import { Button } from '@/components/ui/button';
import { Bell, Menu } from 'lucide-react';
import { useUIStore, useTopicStore } from '@/stores';
import { useNavigate } from '@tanstack/react-router';
import { AccountSwitcher } from '@/components/auth/AccountSwitcher';
import { RealtimeIndicator } from '@/components/RealtimeIndicator';

export function Header() {
  const { toggleSidebar } = useUIStore();
  const { setCurrentTopic } = useTopicStore();
  const navigate = useNavigate();

  return (
    <header
      role="banner"
      className="h-16 border-b bg-background px-6 flex items-center justify-between"
    >
      <div className="flex items-center gap-4">
        <Button variant="ghost" size="icon" onClick={toggleSidebar} aria-label="メニュー切り替え">
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

        <Button variant="ghost" size="icon" aria-label="通知">
          <Bell className="h-5 w-5" />
        </Button>

        <AccountSwitcher />
      </div>
    </header>
  );
}
