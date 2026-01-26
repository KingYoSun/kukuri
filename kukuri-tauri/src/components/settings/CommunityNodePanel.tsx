import { useEffect, useMemo, useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Label } from '@/components/ui/label';
import { Input } from '@/components/ui/input';
import { Textarea } from '@/components/ui/textarea';
import { Button } from '@/components/ui/button';
import { Switch } from '@/components/ui/switch';
import { Badge } from '@/components/ui/badge';
import { Separator } from '@/components/ui/separator';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { communityNodeApi, type CommunityNodeScope } from '@/lib/api/communityNode';
import { errorHandler } from '@/lib/errorHandler';
import { useCommunityNodeStore } from '@/stores/communityNodeStore';
import { toast } from 'sonner';

const keyScopeOptions: Array<{ value: CommunityNodeScope; label: string }> = [
  { value: 'friend_plus', label: 'フレンド+' },
  { value: 'friend', label: 'フレンド' },
  { value: 'invite', label: '招待' },
  { value: 'public', label: '公開' },
];

export function CommunityNodePanel() {
  const queryClient = useQueryClient();
  const [baseUrl, setBaseUrl] = useState('');
  const [syncTopicId, setSyncTopicId] = useState('');
  const [syncScope, setSyncScope] = useState<CommunityNodeScope>('invite');
  const [syncAfterEpoch, setSyncAfterEpoch] = useState('');
  const [inviteJson, setInviteJson] = useState('');
  const {
    enableAccessControl,
    enableLabels,
    enableTrust,
    enableSearch,
    setEnableAccessControl,
    setEnableLabels,
    setEnableTrust,
    setEnableSearch,
  } = useCommunityNodeStore();

  const configQuery = useQuery({
    queryKey: ['community-node', 'config'],
    queryFn: () => communityNodeApi.getConfig(),
    staleTime: 1000 * 60,
  });
  const groupKeysQuery = useQuery({
    queryKey: ['community-node', 'group-keys'],
    queryFn: () => communityNodeApi.listGroupKeys(),
    staleTime: 1000 * 30,
  });
  const consentsQuery = useQuery({
    queryKey: ['community-node', 'consents'],
    queryFn: () => communityNodeApi.getConsentStatus(),
    enabled: Boolean(configQuery.data?.has_token),
    staleTime: 1000 * 30,
  });

  useEffect(() => {
    if (configQuery.data?.base_url) {
      setBaseUrl(configQuery.data.base_url);
    }
  }, [configQuery.data?.base_url]);

  const tokenExpiresAt = useMemo(() => {
    if (!configQuery.data?.token_expires_at) {
      return null;
    }
    const date = new Date(configQuery.data.token_expires_at * 1000);
    return date.toLocaleString();
  }, [configQuery.data?.token_expires_at]);

  const refreshCommunityData = async () => {
    await queryClient.invalidateQueries({ queryKey: ['community-node', 'config'] });
    await queryClient.invalidateQueries({ queryKey: ['community-node', 'group-keys'] });
    await queryClient.invalidateQueries({ queryKey: ['community-node', 'consents'] });
  };

  const handleSaveConfig = async () => {
    try {
      await communityNodeApi.setConfig(baseUrl);
      await refreshCommunityData();
      toast.success('Community Node 設定を更新しました');
    } catch (error) {
      errorHandler.log('Community node config update failed', error, {
        context: 'CommunityNodePanel.saveConfig',
        showToast: true,
        toastTitle: 'Community Node 設定の更新に失敗しました',
      });
    }
  };

  const handleClearConfig = async () => {
    try {
      await communityNodeApi.clearConfig();
      await refreshCommunityData();
      setBaseUrl('');
      toast.success('Community Node 設定をクリアしました');
    } catch (error) {
      errorHandler.log('Community node config clear failed', error, {
        context: 'CommunityNodePanel.clearConfig',
        showToast: true,
        toastTitle: 'Community Node 設定のクリアに失敗しました',
      });
    }
  };

  const handleAuthenticate = async () => {
    try {
      await communityNodeApi.authenticate();
      await refreshCommunityData();
      toast.success('Community Node 認証を更新しました');
    } catch (error) {
      errorHandler.log('Community node auth failed', error, {
        context: 'CommunityNodePanel.authenticate',
        showToast: true,
        toastTitle: 'Community Node 認証に失敗しました',
      });
    }
  };

  const handleClearToken = async () => {
    try {
      await communityNodeApi.clearToken();
      await refreshCommunityData();
      toast.success('Community Node トークンを削除しました');
    } catch (error) {
      errorHandler.log('Community node token clear failed', error, {
        context: 'CommunityNodePanel.clearToken',
        showToast: true,
        toastTitle: 'Community Node トークン削除に失敗しました',
      });
    }
  };

  const handleSyncKeyEnvelopes = async () => {
    if (!syncTopicId.trim()) {
      toast.error('トピックIDを入力してください');
      return;
    }
    try {
      const afterEpoch = syncAfterEpoch.trim() ? Number(syncAfterEpoch.trim()) : undefined;
      await communityNodeApi.syncKeyEnvelopes({
        topic_id: syncTopicId.trim(),
        scope: syncScope,
        after_epoch: Number.isFinite(afterEpoch) ? afterEpoch : undefined,
      });
      await refreshCommunityData();
      toast.success('鍵情報を同期しました');
    } catch (error) {
      errorHandler.log('Community node key sync failed', error, {
        context: 'CommunityNodePanel.syncKeyEnvelopes',
        showToast: true,
        toastTitle: '鍵情報の同期に失敗しました',
      });
    }
  };

  const handleRedeemInvite = async () => {
    if (!inviteJson.trim()) {
      toast.error('招待イベントJSONを入力してください');
      return;
    }
    try {
      const payload = JSON.parse(inviteJson);
      await communityNodeApi.redeemInvite(payload);
      await refreshCommunityData();
      toast.success('招待を適用しました');
    } catch (error) {
      errorHandler.log('Community node invite redeem failed', error, {
        context: 'CommunityNodePanel.redeemInvite',
        showToast: true,
        toastTitle: '招待の適用に失敗しました',
      });
    }
  };

  const handleAcceptConsents = async () => {
    try {
      await communityNodeApi.acceptConsents({ accept_all_current: true });
      await refreshCommunityData();
      toast.success('同意状況を更新しました');
    } catch (error) {
      errorHandler.log('Community node consent update failed', error, {
        context: 'CommunityNodePanel.acceptConsents',
        showToast: true,
        toastTitle: '同意状況の更新に失敗しました',
      });
    }
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle>Community Node</CardTitle>
        <CardDescription>コミュニティノードとの連携設定を管理します。</CardDescription>
      </CardHeader>
      <CardContent className="space-y-6">
        <div className="space-y-3">
          <Label htmlFor="community-node-base-url">Base URL</Label>
          <Input
            id="community-node-base-url"
            data-testid="community-node-base-url"
            placeholder="https://community.example"
            value={baseUrl}
            onChange={(e) => setBaseUrl(e.target.value)}
          />
          <div className="flex flex-wrap items-center gap-2">
            <Button
              variant="outline"
              onClick={handleSaveConfig}
              data-testid="community-node-save-config"
            >
              保存
            </Button>
            <Button
              variant="ghost"
              onClick={handleClearConfig}
              data-testid="community-node-clear-config"
            >
              クリア
            </Button>
          </div>
          <div
            className="flex flex-wrap items-center gap-2 text-sm text-muted-foreground"
            data-testid="community-node-token-status"
            data-has-token={configQuery.data?.has_token ? 'true' : 'false'}
            data-pubkey={configQuery.data?.pubkey ?? ''}
          >
            <span>認証状態:</span>
            {configQuery.data?.has_token ? (
              <Badge variant="secondary">有効</Badge>
            ) : (
              <Badge variant="outline">未認証</Badge>
            )}
            {configQuery.data?.pubkey && <span>pubkey: {configQuery.data.pubkey}</span>}
            {tokenExpiresAt && <span>有効期限: {tokenExpiresAt}</span>}
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <Button
              onClick={handleAuthenticate}
              disabled={!baseUrl.trim()}
              data-testid="community-node-authenticate"
            >
              認証
            </Button>
            <Button
              variant="ghost"
              onClick={handleClearToken}
              disabled={!configQuery.data?.has_token}
              data-testid="community-node-clear-token"
            >
              トークン削除
            </Button>
          </div>
        </div>

        <Separator />

        <div className="space-y-3">
          <div className="flex items-center justify-between">
            <div>
              <p className="font-medium">利用機能</p>
              <p className="text-sm text-muted-foreground">
                Community Node の機能を UI で有効化します。
              </p>
            </div>
          </div>
          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <Label htmlFor="community-node-access-control">アクセス制御 (スコープ)</Label>
              <Switch
                id="community-node-access-control"
                checked={enableAccessControl}
                onCheckedChange={setEnableAccessControl}
              />
            </div>
            <div className="flex items-center justify-between">
              <Label htmlFor="community-node-labels">ラベル/モデレーション</Label>
              <Switch
                id="community-node-labels"
                checked={enableLabels}
                onCheckedChange={setEnableLabels}
              />
            </div>
            <div className="flex items-center justify-between">
              <Label htmlFor="community-node-trust">信頼スコア</Label>
              <Switch
                id="community-node-trust"
                checked={enableTrust}
                onCheckedChange={setEnableTrust}
              />
            </div>
            <div className="flex items-center justify-between">
              <Label htmlFor="community-node-search">検索連携</Label>
              <Switch
                id="community-node-search"
                checked={enableSearch}
                onCheckedChange={setEnableSearch}
              />
            </div>
          </div>
        </div>

        <Separator />

        <div className="space-y-3">
          <p className="font-medium">同意ステータス</p>
          <div
            className="rounded-md border p-3 text-xs text-muted-foreground"
            data-testid="community-node-consents"
          >
            {consentsQuery.data ? JSON.stringify(consentsQuery.data, null, 2) : '未取得'}
          </div>
          <Button
            variant="outline"
            onClick={handleAcceptConsents}
            disabled={!configQuery.data?.has_token}
            data-testid="community-node-accept-consents"
          >
            同意を更新
          </Button>
        </div>

        <Separator />

        <div className="space-y-3">
          <p className="font-medium">鍵エンベロープ同期</p>
          <div className="grid gap-3 md:grid-cols-3">
            <div className="space-y-2 md:col-span-2">
              <Label htmlFor="key-sync-topic-id">トピックID</Label>
              <Input
                id="key-sync-topic-id"
                placeholder="kukuri:topic"
                value={syncTopicId}
                onChange={(e) => setSyncTopicId(e.target.value)}
              />
            </div>
            <div className="space-y-2">
              <Label>スコープ</Label>
              <Select
                value={syncScope}
                onValueChange={(value) => setSyncScope(value as CommunityNodeScope)}
              >
                <SelectTrigger>
                  <SelectValue placeholder="スコープを選択" />
                </SelectTrigger>
                <SelectContent>
                  {keyScopeOptions.map((option) => (
                    <SelectItem key={option.value} value={option.value}>
                      {option.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          </div>
          <div className="space-y-2">
            <Label htmlFor="key-sync-epoch">after_epoch (任意)</Label>
            <Input
              id="key-sync-epoch"
              placeholder="2"
              value={syncAfterEpoch}
              onChange={(e) => setSyncAfterEpoch(e.target.value)}
            />
          </div>
          <Button
            onClick={handleSyncKeyEnvelopes}
            disabled={!configQuery.data?.has_token}
            data-testid="community-node-sync-keys"
          >
            同期
          </Button>
        </div>

        <Separator />

        <div className="space-y-3">
          <p className="font-medium">招待の適用</p>
          <Textarea
            placeholder="invite.capability のイベントJSON"
            rows={6}
            value={inviteJson}
            onChange={(e) => setInviteJson(e.target.value)}
            data-testid="community-node-invite-json"
          />
          <Button
            onClick={handleRedeemInvite}
            disabled={!configQuery.data?.has_token}
            data-testid="community-node-redeem-invite"
          >
            招待を適用
          </Button>
        </div>

        <Separator />

        <div className="space-y-3">
          <p className="font-medium">保存済み鍵</p>
          {groupKeysQuery.data && groupKeysQuery.data.length > 0 ? (
            <div className="space-y-2">
              {groupKeysQuery.data.map((entry) => (
                <div
                  key={`${entry.topic_id}-${entry.scope}-${entry.epoch}`}
                  className="rounded-md border p-3 text-sm"
                >
                  <div className="font-medium">{entry.topic_id}</div>
                  <div className="text-xs text-muted-foreground">
                    scope: {entry.scope} / epoch: {entry.epoch} / stored:{' '}
                    {new Date(entry.stored_at * 1000).toLocaleString()}
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <p className="text-sm text-muted-foreground">保存済み鍵はありません。</p>
          )}
        </div>
      </CardContent>
    </Card>
  );
}
