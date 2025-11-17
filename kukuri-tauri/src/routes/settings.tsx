import { useEffect, useState } from 'react';
import { createFileRoute } from '@tanstack/react-router';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { Button } from '@/components/ui/button';
import { useUIStore, usePrivacySettingsStore } from '@/stores';
import { useAuthStore } from '@/stores/authStore';
import { NostrTestPanel } from '@/components/NostrTestPanel';
import { P2PDebugPanel } from '@/components/P2PDebugPanel';
import { PeerConnectionPanel } from '@/components/p2p/PeerConnectionPanel';
import { BootstrapConfigPanel } from '@/components/p2p/BootstrapConfigPanel';
import { ProfileEditDialog } from '@/components/settings/ProfileEditDialog';
import { toast } from 'sonner';
import { errorHandler } from '@/lib/errorHandler';
import { TauriApi } from '@/lib/api/tauri';
import { updateNostrMetadata } from '@/lib/api/nostr';
import { KeyManagementDialog } from '@/components/settings/KeyManagementDialog';

export const Route = createFileRoute('/settings')({
  component: SettingsPage,
});

function SettingsPage() {
  const { theme, setTheme } = useUIStore();
  const {
    publicProfile,
    showOnlineStatus,
    setPublicProfile,
    setShowOnlineStatus,
    hydrateFromUser: storeHydrateFromUser,
  } = usePrivacySettingsStore();
  const hydrateFromUser = storeHydrateFromUser ?? (() => {});
  const { currentUser, updateUser } = useAuthStore();
  const [isProfileDialogOpen, setProfileDialogOpen] = useState(false);
  const [isKeyDialogOpen, setKeyDialogOpen] = useState(false);
  const [savingField, setSavingField] = useState<'public' | 'online' | null>(null);

  useEffect(() => {
    hydrateFromUser(currentUser ?? null);
  }, [currentUser, hydrateFromUser]);

  const persistPrivacy = async (field: 'public' | 'online', value: boolean) => {
    if (!currentUser) {
      toast.error('プライバシー設定を変更するにはログインが必要です');
      return;
    }
    const payload = {
      publicProfile: field === 'public' ? value : publicProfile,
      showOnlineStatus: field === 'online' ? value : showOnlineStatus,
    };
    setSavingField(field);
    try {
      await TauriApi.updatePrivacySettings({
        npub: currentUser.npub,
        publicProfile: payload.publicProfile,
        showOnlineStatus: payload.showOnlineStatus,
      });
      await updateNostrMetadata({
        kukuri_privacy: {
          public_profile: payload.publicProfile,
          show_online_status: payload.showOnlineStatus,
        },
      });
      updateUser(payload);
      toast.success('プライバシー設定を更新しました');
    } catch (error) {
      errorHandler.log('SettingsPage.updatePrivacyFailed', error, {
        context: 'SettingsPage.persistPrivacy',
        metadata: { field },
      });
      toast.error('プライバシー設定の更新に失敗しました');
      if (field === 'public') {
        setPublicProfile(!value);
      } else {
        setShowOnlineStatus(!value);
      }
    } finally {
      setSavingField(null);
    }
  };

  const handlePrivacyToggle =
    (field: 'public' | 'online') =>
    (checked: boolean): void => {
      if (!currentUser) {
        toast.error('プライバシー設定を変更するにはログインが必要です');
        return;
      }
      if (field === 'public') {
        setPublicProfile(checked);
      } else {
        setShowOnlineStatus(checked);
      }
      void persistPrivacy(field, checked);
    };

  return (
    <div className="max-w-2xl mx-auto space-y-6">
      <h1 className="text-3xl font-bold">設定</h1>

      <Card>
        <CardHeader>
          <CardTitle>外観</CardTitle>
          <CardDescription>アプリケーションの見た目をカスタマイズします</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex items-center justify-between">
            <Label htmlFor="dark-mode">ダークモード</Label>
            <Switch
              id="dark-mode"
              checked={theme === 'dark'}
              onCheckedChange={(checked) => setTheme(checked ? 'dark' : 'light')}
            />
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>アカウント</CardTitle>
          <CardDescription>アカウント情報と設定を管理します</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex items-center justify-between">
            <div>
              <p className="font-medium">プロフィール編集</p>
              <p className="text-sm text-muted-foreground">表示名、自己紹介、アバター画像を編集</p>
            </div>
            <Button variant="outline" onClick={() => setProfileDialogOpen(true)}>
              編集
            </Button>
          </div>
          <div className="flex items-center justify-between">
            <div>
              <p className="font-medium">鍵管理</p>
              <p className="text-sm text-muted-foreground">秘密鍵のバックアップとインポート</p>
            </div>
            <Button variant="outline" onClick={() => setKeyDialogOpen(true)}>
              管理
            </Button>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>プライバシー</CardTitle>
          <CardDescription>プライバシー設定を管理します</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex items-center justify-between">
            <Label htmlFor="public-profile">プロフィールを公開</Label>
            <Switch
              id="public-profile"
              checked={publicProfile}
              disabled={!currentUser || savingField === 'public'}
              onCheckedChange={handlePrivacyToggle('public')}
            />
          </div>
          <div className="flex items-center justify-between">
            <Label htmlFor="show-online">オンライン状態を表示</Label>
            <Switch
              id="show-online"
              checked={showOnlineStatus}
              disabled={!currentUser || savingField === 'online'}
              onCheckedChange={handlePrivacyToggle('online')}
            />
          </div>
          {!currentUser && (
            <p className="text-xs text-muted-foreground">
              ログインするとプライバシー設定を変更できます
            </p>
          )}
          {savingField && (
            <p className="text-xs text-muted-foreground">プライバシー設定を保存しています…</p>
          )}
        </CardContent>
      </Card>

      <PeerConnectionPanel />
      <BootstrapConfigPanel />

      {/* 開発環境でのみ表示 */}
      {import.meta.env.DEV && (
        <>
          <Card>
            <CardHeader>
              <CardTitle>開発者ツール - Nostr</CardTitle>
              <CardDescription>Nostrプロトコルのテストとデバッグ</CardDescription>
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
