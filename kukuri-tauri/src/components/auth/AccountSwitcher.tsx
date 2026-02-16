import { useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';

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
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog';
import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar';
import { ChevronDown, User, LogOut, Trash2 } from 'lucide-react';
import { errorHandler } from '@/lib/errorHandler';
import { useNavigate } from '@tanstack/react-router';
import { resolveAvatarSrc, resolveUserAvatarSrc } from '@/lib/profile/avatarDisplay';

export function AccountSwitcher() {
  const { t } = useTranslation();
  const { currentUser, accounts, switchAccount, removeAccount, logout } = useAuthStore();
  const navigate = useNavigate();
  const isSwitchingRef = useRef(false);
  const [showDeleteDialog, setShowDeleteDialog] = useState(false);
  const [accountToDelete, setAccountToDelete] = useState<string | null>(null);

  if (!currentUser) {
    return null;
  }

  const currentUserAvatarSrc = resolveUserAvatarSrc(currentUser);

  const getInitials = (name: string) => {
    return name
      .split(' ')
      .map((n) => n[0])
      .join('')
      .toUpperCase()
      .slice(0, 2);
  };

  const labelText = currentUser.displayName || currentUser.name || currentUser.npub;

  const handleSwitchAccount = async (npub: string) => {
    if (isSwitchingRef.current) {
      return;
    }
    isSwitchingRef.current = true;
    try {
      await switchAccount(npub);
    } catch (error) {
      errorHandler.log('Failed to switch account', error, {
        context: 'AccountSwitcher.handleSwitchAccount',
      });
    } finally {
      isSwitchingRef.current = false;
    }
  };

  const handleRemoveAccountClick = (npub: string) => {
    setAccountToDelete(npub);
    setShowDeleteDialog(true);
  };

  const handleConfirmDelete = async () => {
    if (!accountToDelete) return;
    try {
      await removeAccount(accountToDelete);
    } catch (error) {
      errorHandler.log('Failed to remove account', error, {
        context: 'AccountSwitcher.handleConfirmDelete',
      });
    } finally {
      setShowDeleteDialog(false);
      setAccountToDelete(null);
    }
  };

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          variant="ghost"
          className="flex items-center gap-2"
          data-testid="account-switcher-trigger"
          title={labelText}
          aria-label={labelText}
        >
          <Avatar className="h-8 w-8">
            <AvatarImage src={currentUserAvatarSrc} alt={currentUser.displayName} />
            <AvatarFallback>{getInitials(currentUser.displayName)}</AvatarFallback>
          </Avatar>
          <span className="max-w-[150px] truncate" data-testid="account-switcher-trigger-text">
            {labelText}
          </span>
          <ChevronDown className="h-4 w-4" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-64">
        <DropdownMenuLabel>{t('common.account')}</DropdownMenuLabel>
        <DropdownMenuSeparator />

        {/* 現在のアカウント */}
        <DropdownMenuItem disabled className="opacity-100">
          <div className="flex items-center gap-3 w-full">
            <Avatar className="h-8 w-8">
              <AvatarImage src={currentUserAvatarSrc} alt={currentUser.displayName} />
              <AvatarFallback>{getInitials(currentUser.displayName)}</AvatarFallback>
            </Avatar>
            <div className="flex-1 overflow-hidden">
              <p className="text-sm font-medium truncate">{currentUser.displayName}</p>
              <p className="text-xs text-muted-foreground truncate">{currentUser.npub}</p>
            </div>
            <div className="text-xs text-muted-foreground">{t('common.current')}</div>
          </div>
        </DropdownMenuItem>

        <DropdownMenuSeparator />

        {/* 他のアカウント */}
        {accounts
          .filter((account) => account.npub !== currentUser.npub)
          .map((account) => (
            <DropdownMenuItem
              key={account.npub}
              className="cursor-pointer"
              onSelect={() => handleSwitchAccount(account.npub)}
              onClick={() => handleSwitchAccount(account.npub)}
              data-testid="account-switch-option"
              aria-label={`${account.display_name || account.name || 'account'} (${account.npub})`}
              data-account-npub={account.npub}
              data-account-display-name={account.display_name}
            >
              <div className="flex items-center gap-3 w-full">
                <Avatar className="h-8 w-8">
                  <AvatarImage src={resolveAvatarSrc(account.picture)} alt={account.display_name} />
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
            <p className="text-sm text-muted-foreground">{t('common.noOtherAccounts')}</p>
          </DropdownMenuItem>
        )}

        <DropdownMenuSeparator />

        {/* アクション */}
        <DropdownMenuItem
          onSelect={() => navigate({ to: '/login' })}
          data-testid="account-menu-go-login"
        >
          <User className="mr-2 h-4 w-4" />
          <span>{t('common.addAnotherAccount')}</span>
        </DropdownMenuItem>

        <DropdownMenuItem
          onSelect={(e) => {
            e.preventDefault();
            handleRemoveAccountClick(currentUser.npub);
          }}
          className="text-destructive"
          data-testid="account-menu-remove-current"
        >
          <Trash2 className="mr-2 h-4 w-4" />
          <span>{t('common.removeAccount')}</span>
        </DropdownMenuItem>

        <DropdownMenuItem onSelect={logout} data-testid="account-menu-logout">
          <LogOut className="mr-2 h-4 w-4" />
          <span>{t('common.logout')}</span>
        </DropdownMenuItem>
      </DropdownMenuContent>
      <AlertDialog open={showDeleteDialog} onOpenChange={setShowDeleteDialog}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>{t('common.removeAccount')}</AlertDialogTitle>
            <AlertDialogDescription>{t('common.removeAccountConfirm')}</AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>{t('common.cancel')}</AlertDialogCancel>
            <AlertDialogAction
              onClick={handleConfirmDelete}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
              {t('common.delete')}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </DropdownMenu>
  );
}
