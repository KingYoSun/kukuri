import { useCallback, useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { createFileRoute } from '@tanstack/react-router';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { Button } from '@/components/ui/button';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { useUIStore, usePrivacySettingsStore } from '@/stores';
import { useOfflineStore } from '@/stores/offlineStore';
import { useAuthStore } from '@/stores/authStore';
import { NostrTestPanel } from '@/components/NostrTestPanel';
import { P2PDebugPanel } from '@/components/P2PDebugPanel';
import { PeerConnectionPanel } from '@/components/p2p/PeerConnectionPanel';
import { BootstrapConfigPanel } from '@/components/p2p/BootstrapConfigPanel';
import { ProfileEditDialog } from '@/components/settings/ProfileEditDialog';
import { CommunityNodePanel } from '@/components/settings/CommunityNodePanel';
import { toast } from 'sonner';
import { errorHandler } from '@/lib/errorHandler';
import { KeyManagementDialog } from '@/components/settings/KeyManagementDialog';
import {
  syncPrivacySettings,
  type PrivacySettingsSyncPayload,
} from '@/lib/settings/privacySettingsSync';
import { SUPPORTED_LOCALES, getCurrentLocale, persistLocale, type SupportedLocale } from '@/i18n';

export const Route = createFileRoute('/settings')({
  component: SettingsPage,
});

function SettingsPage() {
  const { t, i18n } = useTranslation();
  const { theme, setTheme } = useUIStore();
  const {
    publicProfile,
    showOnlineStatus,
    hasPendingSync,
    applyLocalChange,
    markSyncSuccess,
    markSyncFailure,
    hydrateFromUser: storeHydrateFromUser,
  } = usePrivacySettingsStore();
  const hydrateFromUser = storeHydrateFromUser ?? (() => {});
  const { currentUser, updateUser } = useAuthStore();
  const isOnline = useOfflineStore((state) => state.isOnline);
  const [isProfileDialogOpen, setProfileDialogOpen] = useState(false);
  const [isKeyDialogOpen, setKeyDialogOpen] = useState(false);
  const [isSyncingPrivacy, setIsSyncingPrivacy] = useState(false);
  const isSyncingPrivacyRef = useRef(false);

  useEffect(() => {
    hydrateFromUser(currentUser ?? null);
  }, [currentUser, hydrateFromUser]);

  const persistPrivacy = useCallback(
    async (payload: PrivacySettingsSyncPayload) => {
      isSyncingPrivacyRef.current = true;
      setIsSyncingPrivacy(true);
      try {
        await syncPrivacySettings(payload);
        markSyncSuccess();
        updateUser({
          publicProfile: payload.publicProfile,
          showOnlineStatus: payload.showOnlineStatus,
        });
        toast.success(t('settings.toast.privacyUpdated'));
      } catch (error) {
        markSyncFailure(error instanceof Error ? error.message : null);
        errorHandler.log('SettingsPage.updatePrivacyFailed', error, {
          context: 'SettingsPage.persistPrivacy',
          metadata: { npub: payload.npub },
        });
        toast.error(t('settings.toast.privacyUpdateFailed'));
      } finally {
        isSyncingPrivacyRef.current = false;
        setIsSyncingPrivacy(false);
      }
    },
    [markSyncFailure, markSyncSuccess, t, updateUser],
  );

  const syncPendingPrivacyIfNeeded = useCallback(() => {
    const latestUser = useAuthStore.getState().currentUser;
    const latestOffline = useOfflineStore.getState();
    const latestPrivacy = usePrivacySettingsStore.getState();
    if (
      !latestUser ||
      !latestOffline.isOnline ||
      !latestPrivacy.hasPendingSync ||
      isSyncingPrivacyRef.current
    ) {
      return;
    }
    void persistPrivacy({
      npub: latestUser.npub,
      publicProfile: latestPrivacy.publicProfile,
      showOnlineStatus: latestPrivacy.showOnlineStatus,
    });
  }, [persistPrivacy]);

  useEffect(() => {
    syncPendingPrivacyIfNeeded();
    const handleOnline = () => {
      syncPendingPrivacyIfNeeded();
    };
    window.addEventListener('online', handleOnline);
    return () => {
      window.removeEventListener('online', handleOnline);
    };
  }, [syncPendingPrivacyIfNeeded]);

  const handlePrivacyToggle =
    (field: 'public' | 'online') =>
    (checked: boolean): void => {
      if (!currentUser) {
        toast.error(t('settings.toast.privacyLoginRequired'));
        return;
      }
      const payload: PrivacySettingsSyncPayload = {
        npub: currentUser.npub,
        publicProfile: field === 'public' ? checked : publicProfile,
        showOnlineStatus: field === 'online' ? checked : showOnlineStatus,
      };
      applyLocalChange(payload);
      updateUser({
        publicProfile: payload.publicProfile,
        showOnlineStatus: payload.showOnlineStatus,
      });
      if (!isOnline) {
        toast.info(t('settings.toast.privacySavedOffline'));
        return;
      }
      void persistPrivacy(payload);
    };

  const currentLocale = getCurrentLocale();
  const handleLocaleChange = (value: string) => {
    const locale = value as SupportedLocale;
    persistLocale(locale);
    void i18n.changeLanguage(locale);
  };

  return (
    <div className="max-w-2xl mx-auto space-y-6" data-testid="settings-page">
      <h1 className="text-3xl font-bold">{t('settings.title')}</h1>

      <Card>
        <CardHeader>
          <CardTitle>{t('settings.appearance.title')}</CardTitle>
          <CardDescription>{t('settings.appearance.description')}</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex items-center justify-between">
            <Label htmlFor="dark-mode">{t('settings.appearance.darkMode')}</Label>
            <Switch
              id="dark-mode"
              checked={theme === 'dark'}
              onCheckedChange={(checked) => setTheme(checked ? 'dark' : 'light')}
            />
          </div>
          <div className="flex items-center justify-between">
            <div>
              <Label htmlFor="language" className="text-base font-medium">
                {t('settings.appearance.language')}
              </Label>
              <p className="text-sm text-muted-foreground">
                {t('settings.appearance.languageDescription')}
              </p>
            </div>
            <Select
              value={currentLocale}
              onValueChange={handleLocaleChange}
              data-testid="settings-language-select"
            >
              <SelectTrigger id="language" className="w-[180px]">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {SUPPORTED_LOCALES.map((locale) => (
                  <SelectItem key={locale} value={locale}>
                    {t(`common.language.${locale === 'zh-CN' ? 'zhCN' : locale}`)}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>{t('settings.account.title')}</CardTitle>
          <CardDescription>{t('settings.account.description')}</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex items-center justify-between">
            <div>
              <p className="font-medium">{t('settings.account.profileEdit')}</p>
              <p className="text-sm text-muted-foreground">
                {t('settings.account.profileEditDescription')}
              </p>
            </div>
            <Button
              variant="outline"
              onClick={() => setProfileDialogOpen(true)}
              data-testid="open-profile-dialog"
            >
              {t('settings.account.edit')}
            </Button>
          </div>
          <div className="flex items-center justify-between">
            <div>
              <p className="font-medium">{t('settings.account.keyManagement')}</p>
              <p className="text-sm text-muted-foreground">
                {t('settings.account.keyManagementDescription')}
              </p>
            </div>
            <Button
              variant="outline"
              onClick={() => setKeyDialogOpen(true)}
              data-testid="open-key-dialog"
            >
              {t('settings.account.manage')}
            </Button>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>{t('settings.privacy.title')}</CardTitle>
          <CardDescription>{t('settings.privacy.description')}</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex items-center justify-between">
            <Label htmlFor="public-profile">{t('settings.privacy.publicProfile')}</Label>
            <Switch
              id="public-profile"
              checked={publicProfile}
              disabled={!currentUser || isSyncingPrivacy}
              onCheckedChange={handlePrivacyToggle('public')}
            />
          </div>
          <div className="flex items-center justify-between">
            <Label htmlFor="show-online">{t('settings.privacy.showOnlineStatus')}</Label>
            <Switch
              id="show-online"
              checked={showOnlineStatus}
              disabled={!currentUser || isSyncingPrivacy}
              onCheckedChange={handlePrivacyToggle('online')}
            />
          </div>
          {!currentUser && (
            <p className="text-xs text-muted-foreground">{t('settings.privacy.loginRequired')}</p>
          )}
          {isSyncingPrivacy && (
            <p className="text-xs text-muted-foreground">{t('settings.privacy.saving')}</p>
          )}
          {!isSyncingPrivacy && currentUser && hasPendingSync && (
            <p className="text-xs text-muted-foreground">{t('settings.privacy.pendingSync')}</p>
          )}
        </CardContent>
      </Card>

      <PeerConnectionPanel />
      <BootstrapConfigPanel />
      <CommunityNodePanel />

      {/* 開発環境でのみ表示 */}
      {import.meta.env.DEV && (
        <>
          <Card>
            <CardHeader>
              <CardTitle>{t('settings.devTools.nostrTitle')}</CardTitle>
              <CardDescription>{t('settings.devTools.nostrDescription')}</CardDescription>
            </CardHeader>
            <CardContent>
              <NostrTestPanel />
            </CardContent>
          </Card>

          <P2PDebugPanel />
        </>
      )}

      <ProfileEditDialog
        open={isProfileDialogOpen}
        onOpenChange={(open) => setProfileDialogOpen(open)}
      />
      <KeyManagementDialog open={isKeyDialogOpen} onOpenChange={setKeyDialogOpen} />
    </div>
  );
}

export { SettingsPage };
