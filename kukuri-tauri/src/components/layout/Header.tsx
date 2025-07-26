import { Button } from '@/components/ui/button';
import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { Bell, Settings, LogOut, Menu } from 'lucide-react';
import { useAuthStore, useUIStore } from '@/stores';
import { useNavigate } from '@tanstack/react-router';

export function Header() {
  const { currentUser, logout } = useAuthStore();
  const { toggleSidebar } = useUIStore();
  const navigate = useNavigate();

  const handleLogout = () => {
    logout();
  };

  const handleSettings = () => {
    navigate({ to: '/settings' });
  };

  const getUserInitials = () => {
    if (!currentUser?.name) return 'U';
    return currentUser.name.slice(0, 2).toUpperCase();
  };

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
          onClick={() => navigate({ to: '/' })}
        >
          kukuri
        </h1>
      </div>

      <div className="flex items-center gap-4">
        <Button variant="ghost" size="icon" aria-label="通知">
          <Bell className="h-5 w-5" />
        </Button>

        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button variant="ghost" className="relative h-10 w-10 rounded-full">
              <Avatar className="h-10 w-10">
                <AvatarImage src={currentUser?.picture} alt={currentUser?.name || 'User'} />
                <AvatarFallback>{getUserInitials()}</AvatarFallback>
              </Avatar>
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent className="w-56" align="end" forceMount>
            <DropdownMenuLabel>{currentUser?.name || 'マイアカウント'}</DropdownMenuLabel>
            <DropdownMenuSeparator />
            <DropdownMenuItem onClick={handleSettings}>
              <Settings className="mr-2 h-4 w-4" />
              <span>設定</span>
            </DropdownMenuItem>
            <DropdownMenuItem onClick={handleLogout}>
              <LogOut className="mr-2 h-4 w-4" />
              <span>ログアウト</span>
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
    </header>
  );
}
