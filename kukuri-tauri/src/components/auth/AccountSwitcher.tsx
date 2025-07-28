import React from 'react';
import { useAuthStore } from '@/stores/authStore';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar';
import { ChevronDown, User, LogOut, Trash2 } from 'lucide-react';

export function AccountSwitcher() {
  const { currentUser, accounts, switchAccount, removeAccount, logout } = useAuthStore();

  if (!currentUser) {
    return null;
  }

  const getInitials = (name: string) => {
    return name
      .split(' ')
      .map(n => n[0])
      .join('')
      .toUpperCase()
      .slice(0, 2);
  };

  const handleSwitchAccount = async (npub: string) => {
    try {
      await switchAccount(npub);
    } catch (error) {
      console.error('Failed to switch account:', error);
    }
  };

  const handleRemoveAccount = async (npub: string) => {
    if (confirm('このアカウントを削除してもよろしいですか？')) {
      try {
        await removeAccount(npub);
      } catch (error) {
        console.error('Failed to remove account:', error);
      }
    }
  };

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="ghost" className="flex items-center gap-2">
          <Avatar className="h-8 w-8">
            <AvatarImage src={currentUser.picture} alt={currentUser.displayName} />
            <AvatarFallback>{getInitials(currentUser.displayName)}</AvatarFallback>
          </Avatar>
          <span className="max-w-[150px] truncate">{currentUser.displayName}</span>
          <ChevronDown className="h-4 w-4" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-64">
        <DropdownMenuLabel>アカウント</DropdownMenuLabel>
        <DropdownMenuSeparator />
        
        {/* 現在のアカウント */}
        <DropdownMenuItem disabled className="opacity-100">
          <div className="flex items-center gap-3 w-full">
            <Avatar className="h-8 w-8">
              <AvatarImage src={currentUser.picture} alt={currentUser.displayName} />
              <AvatarFallback>{getInitials(currentUser.displayName)}</AvatarFallback>
            </Avatar>
            <div className="flex-1 overflow-hidden">
              <p className="text-sm font-medium truncate">{currentUser.displayName}</p>
              <p className="text-xs text-muted-foreground truncate">{currentUser.npub}</p>
            </div>
            <div className="text-xs text-muted-foreground">現在</div>
          </div>
        </DropdownMenuItem>
        
        <DropdownMenuSeparator />
        
        {/* 他のアカウント */}
        {accounts
          .filter(account => account.npub !== currentUser.npub)
          .map(account => (
            <DropdownMenuItem
              key={account.npub}
              className="cursor-pointer"
              onSelect={() => handleSwitchAccount(account.npub)}
            >
              <div className="flex items-center gap-3 w-full">
                <Avatar className="h-8 w-8">
                  <AvatarImage src={account.picture} alt={account.display_name} />
                  <AvatarFallback>{getInitials(account.display_name)}</AvatarFallback>
                </Avatar>
                <div className="flex-1 overflow-hidden">
                  <p className="text-sm font-medium truncate">{account.display_name}</p>
                  <p className="text-xs text-muted-foreground truncate">{account.npub}</p>
                </div>
              </div>
            </DropdownMenuItem>
          ))}
        
        {accounts.length === 1 && (
          <DropdownMenuItem disabled>
            <p className="text-sm text-muted-foreground">他のアカウントがありません</p>
          </DropdownMenuItem>
        )}
        
        <DropdownMenuSeparator />
        
        {/* アクション */}
        <DropdownMenuItem onSelect={() => window.location.href = '/login'}>
          <User className="mr-2 h-4 w-4" />
          <span>別のアカウントを追加</span>
        </DropdownMenuItem>
        
        <DropdownMenuItem
          onSelect={() => handleRemoveAccount(currentUser.npub)}
          className="text-destructive"
        >
          <Trash2 className="mr-2 h-4 w-4" />
          <span>アカウントを削除</span>
        </DropdownMenuItem>
        
        <DropdownMenuItem onSelect={logout}>
          <LogOut className="mr-2 h-4 w-4" />
          <span>ログアウト</span>
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}