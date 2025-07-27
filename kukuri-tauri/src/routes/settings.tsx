import { createFileRoute } from '@tanstack/react-router';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { Button } from '@/components/ui/button';
import { useUIStore } from '@/stores';
import { NostrTestPanel } from '@/components/NostrTestPanel';
import { P2PDebugPanel } from '@/components/P2PDebugPanel';

export const Route = createFileRoute('/settings')({
  component: SettingsPage,
});

function SettingsPage() {
  const { theme, setTheme } = useUIStore();

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
            <Button variant="outline">編集</Button>
          </div>
          <div className="flex items-center justify-between">
            <div>
              <p className="font-medium">鍵管理</p>
              <p className="text-sm text-muted-foreground">秘密鍵のバックアップとインポート</p>
            </div>
            <Button variant="outline">管理</Button>
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
            <Switch id="public-profile" defaultChecked />
          </div>
          <div className="flex items-center justify-between">
            <Label htmlFor="show-online">オンライン状態を表示</Label>
            <Switch id="show-online" />
          </div>
        </CardContent>
      </Card>

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
    </div>
  );
}
