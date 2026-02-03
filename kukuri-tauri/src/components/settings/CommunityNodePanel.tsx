import { useEffect, useState } from 'react';
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
import { accessControlApi } from '@/lib/api/accessControl';
import {
  communityNodeApi,
  defaultCommunityNodeRoles,
  type CommunityNodeConfigNodeResponse,
  type CommunityNodeRoleKey,
} from '@/lib/api/communityNode';
import { errorHandler } from '@/lib/errorHandler';
import { useCommunityNodeStore } from '@/stores/communityNodeStore';
import { toast } from 'sonner';

const shortenPubkey = (value: string) =>
  value.length > 16 ? `${value.slice(0, 8)}...${value.slice(-4)}` : value;

const formatAttesterLabel = (node: CommunityNodeConfigNodeResponse) => {
  if (node.pubkey) {
    return `${node.base_url} (${shortenPubkey(node.pubkey)})`;
  }
  return node.base_url;
};

export function CommunityNodePanel() {
  const queryClient = useQueryClient();
  const [newBaseUrl, setNewBaseUrl] = useState('');
  const [actionNodeBaseUrl, setActionNodeBaseUrl] = useState('');
  const [inviteJson, setInviteJson] = useState('');
  const [trustAnchorAttester, setTrustAnchorAttester] = useState('auto');
  const { enableAccessControl, setEnableAccessControl } = useCommunityNodeStore();

  const configQuery = useQuery({
    queryKey: ['community-node', 'config'],
    queryFn: () => communityNodeApi.getConfig(),
    staleTime: 1000 * 60,
  });
  const trustAnchorQuery = useQuery({
    queryKey: ['community-node', 'trust-anchor'],
    queryFn: () => communityNodeApi.getTrustAnchor(),
    staleTime: 1000 * 60,
  });
  const groupKeysQuery = useQuery({
    queryKey: ['community-node', 'group-keys'],
    queryFn: () => communityNodeApi.listGroupKeys(),
    staleTime: 1000 * 30,
  });
  const nodes = configQuery.data?.nodes ?? [];
  const selectedNode =
    nodes.find((node) => node.base_url === actionNodeBaseUrl) ?? nodes[0] ?? null;
  const trustNodes = nodes.filter((node) => node.roles.trust && node.has_token);
  const trustAttesterOptions = trustNodes
    .filter((node) => node.pubkey)
    .map((node) => ({
      value: node.pubkey ?? '',
      label: formatAttesterLabel(node),
      baseUrl: node.base_url,
    }));
  const trustAnchorNode =
    trustAnchorAttester !== 'auto'
      ? (trustNodes.find((node) => node.pubkey === trustAnchorAttester) ?? null)
      : null;

  const consentsQuery = useQuery({
    queryKey: ['community-node', 'consents', selectedNode?.base_url ?? ''],
    queryFn: () =>
      selectedNode
        ? communityNodeApi.getConsentStatus(selectedNode.base_url)
        : Promise.resolve(null),
    enabled: Boolean(selectedNode?.has_token),
    staleTime: 1000 * 30,
  });

  useEffect(() => {
    if (nodes.length === 0) {
      setActionNodeBaseUrl('');
      return;
    }
    const exists = nodes.some((node) => node.base_url === actionNodeBaseUrl);
    if (!exists) {
      setActionNodeBaseUrl(nodes[0].base_url);
    }
  }, [nodes, actionNodeBaseUrl]);

  useEffect(() => {
    if (trustAnchorQuery.data?.attester) {
      setTrustAnchorAttester(trustAnchorQuery.data.attester);
      return;
    }
    setTrustAnchorAttester('auto');
  }, [trustAnchorQuery.data?.attester]);

  useEffect(() => {
    if (trustAnchorQuery.isError) {
      errorHandler.log('Failed to load community node trust anchor', trustAnchorQuery.error, {
        context: 'CommunityNodePanel.trustAnchor',
        showToast: true,
        toastTitle: 'Trust Anchor の取得に失敗しました',
      });
    }
  }, [trustAnchorQuery.isError, trustAnchorQuery.error]);

  const refreshCommunityData = async () => {
    await queryClient.invalidateQueries({ queryKey: ['community-node', 'config'] });
    await queryClient.invalidateQueries({ queryKey: ['community-node', 'group-keys'] });
    await queryClient.invalidateQueries({ queryKey: ['community-node', 'consents'] });
    await queryClient.invalidateQueries({ queryKey: ['community-node', 'trust-anchor'] });
  };

  const serializeNodes = (items: CommunityNodeConfigNodeResponse[]) =>
    items.map((item) => ({ base_url: item.base_url, roles: item.roles }));

  const handleAddNode = async () => {
    const trimmed = newBaseUrl.trim();
    if (!trimmed) {
      toast.error('Base URLを入力してください');
      return;
    }
    const normalized = trimmed.replace(/\/+$/, '');
    if (nodes.some((node) => node.base_url === normalized)) {
      toast.error('同じBase URLのノードが既に登録されています');
      return;
    }
    try {
      const nextNodes = [
        ...serializeNodes(nodes),
        {
          base_url: normalized,
          roles: defaultCommunityNodeRoles,
        },
      ];
      await communityNodeApi.setConfig(nextNodes);
      await refreshCommunityData();
      setNewBaseUrl('');
      toast.success('Community Node を追加しました');
    } catch (error) {
      errorHandler.log('Community node config update failed', error, {
        context: 'CommunityNodePanel.addNode',
        showToast: true,
        toastTitle: 'Community Node の追加に失敗しました',
      });
    }
  };

  const handleClearConfig = async () => {
    try {
      await communityNodeApi.clearConfig();
      await refreshCommunityData();
      setNewBaseUrl('');
      setActionNodeBaseUrl('');
      toast.success('Community Node 設定をクリアしました');
    } catch (error) {
      errorHandler.log('Community node config clear failed', error, {
        context: 'CommunityNodePanel.clearConfig',
        showToast: true,
        toastTitle: 'Community Node 設定のクリアに失敗しました',
      });
    }
  };

  const handleRemoveNode = async (baseUrl: string) => {
    try {
      const nextNodes = nodes.filter((node) => node.base_url !== baseUrl);
      await communityNodeApi.setConfig(serializeNodes(nextNodes));
      await refreshCommunityData();
      toast.success('Community Node を削除しました');
    } catch (error) {
      errorHandler.log('Community node remove failed', error, {
        context: 'CommunityNodePanel.removeNode',
        showToast: true,
        toastTitle: 'Community Node の削除に失敗しました',
      });
    }
  };

  const handleAuthenticate = async (baseUrl: string) => {
    try {
      await communityNodeApi.authenticate(baseUrl);
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

  const handleClearToken = async (baseUrl: string) => {
    try {
      await communityNodeApi.clearToken(baseUrl);
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

  const handleRoleToggle = async (baseUrl: string, role: CommunityNodeRoleKey, value: boolean) => {
    const nextNodes = nodes.map((node) =>
      node.base_url === baseUrl ? { ...node, roles: { ...node.roles, [role]: value } } : node,
    );
    try {
      await communityNodeApi.setConfig(serializeNodes(nextNodes));
      await refreshCommunityData();
    } catch (error) {
      errorHandler.log('Community node role update failed', error, {
        context: 'CommunityNodePanel.updateRoles',
        showToast: true,
        toastTitle: 'Community Node ロール更新に失敗しました',
      });
    }
  };

  const handleTrustAnchorChange = async (value: string) => {
    setTrustAnchorAttester(value);
    try {
      if (value === 'auto') {
        await communityNodeApi.clearTrustAnchor();
      } else {
        await communityNodeApi.setTrustAnchor({ attester: value, weight: 1 });
      }
      await queryClient.invalidateQueries({ queryKey: ['community-node', 'trust-anchor'] });
      await queryClient.invalidateQueries({ queryKey: ['community-node', 'trust'] });
    } catch (error) {
      errorHandler.log('Community node trust anchor update failed', error, {
        context: 'CommunityNodePanel.trustAnchorUpdate',
        showToast: true,
        toastTitle: 'Trust Anchor の更新に失敗しました',
      });
    }
  };

  const handleRequestJoin = async () => {
    if (!inviteJson.trim()) {
      toast.error('招待イベントJSONを入力してください');
      return;
    }
    try {
      const payload = JSON.parse(inviteJson);
      await accessControlApi.requestJoin({
        invite_event_json: payload,
      });
      await refreshCommunityData();
      toast.success('P2P 参加リクエストを送信しました');
    } catch (error) {
      errorHandler.log('Community node join request failed', error, {
        context: 'CommunityNodePanel.requestJoin',
        showToast: true,
        toastTitle: 'P2P 参加リクエストに失敗しました',
      });
    }
  };

  const handleAcceptConsents = async () => {
    if (!selectedNode?.base_url) {
      toast.error('操作対象のノードを選択してください');
      return;
    }
    try {
      await communityNodeApi.acceptConsents({
        base_url: selectedNode.base_url,
        accept_all_current: true,
      });
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
            value={newBaseUrl}
            onChange={(e) => setNewBaseUrl(e.target.value)}
          />
          <div className="flex flex-wrap items-center gap-2">
            <Button
              variant="outline"
              onClick={handleAddNode}
              data-testid="community-node-save-config"
            >
              追加
            </Button>
            <Button
              variant="ghost"
              onClick={handleClearConfig}
              data-testid="community-node-clear-config"
              disabled={nodes.length === 0}
            >
              全削除
            </Button>
          </div>
        </div>

        <Separator />

        <div className="space-y-3">
          <div>
            <p className="font-medium">Trust Anchor (attester)</p>
            <p className="text-sm text-muted-foreground">trust に使う attester を選択します。</p>
          </div>
          <Select
            value={trustAnchorAttester}
            onValueChange={handleTrustAnchorChange}
            disabled={trustAttesterOptions.length === 0}
          >
            <SelectTrigger data-testid="community-node-trust-anchor">
              <SelectValue placeholder="自動" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="auto">自動 (複数ノード平均)</SelectItem>
              {trustAttesterOptions.map((option) => (
                <SelectItem key={option.value} value={option.value}>
                  {option.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          {trustAnchorAttester === 'auto' ? (
            <p className="text-xs text-muted-foreground">
              自動モード: trust ノードの平均値を使用します。
            </p>
          ) : trustAnchorNode ? (
            <p
              className="text-xs text-muted-foreground"
              data-testid="community-node-trust-anchor-current"
            >
              attester: {formatAttesterLabel(trustAnchorNode)}
            </p>
          ) : (
            <p className="text-xs text-muted-foreground">
              選択中の attester が見つかりません。認証済みの trust ノードを選択してください。
            </p>
          )}
        </div>

        <Separator />

        <div className="space-y-3">
          <div className="flex items-center justify-between">
            <div>
              <p className="font-medium">採用ノード</p>
              <p className="text-sm text-muted-foreground">
                role ごとに採用するノードを切り替えます。
              </p>
            </div>
          </div>
          {nodes.length > 0 ? (
            <div className="space-y-3">
              {nodes.map((node, index) => {
                const expiresAt = node.token_expires_at
                  ? new Date(node.token_expires_at * 1000).toLocaleString()
                  : null;
                return (
                  <div
                    key={node.base_url}
                    className="rounded-md border p-3 space-y-3"
                    data-testid={`community-node-node-${index}`}
                  >
                    <div className="flex flex-wrap items-center justify-between gap-2">
                      <div className="text-sm font-medium break-all">{node.base_url}</div>
                      <div className="flex flex-wrap items-center gap-2">
                        <Button
                          size="sm"
                          onClick={() => handleAuthenticate(node.base_url)}
                          data-testid={`community-node-authenticate-${index}`}
                        >
                          認証
                        </Button>
                        <Button
                          size="sm"
                          variant="ghost"
                          onClick={() => handleClearToken(node.base_url)}
                          disabled={!node.has_token}
                          data-testid={`community-node-clear-token-${index}`}
                        >
                          トークン削除
                        </Button>
                        <Button
                          size="sm"
                          variant="ghost"
                          onClick={() => handleRemoveNode(node.base_url)}
                          data-testid={`community-node-remove-${index}`}
                        >
                          削除
                        </Button>
                      </div>
                    </div>
                    <div
                      className="flex flex-wrap items-center gap-2 text-xs text-muted-foreground"
                      data-testid={`community-node-token-status-${index}`}
                      data-has-token={node.has_token ? 'true' : 'false'}
                      data-pubkey={node.pubkey ?? ''}
                    >
                      <span>認証状態:</span>
                      {node.has_token ? (
                        <Badge variant="secondary">有効</Badge>
                      ) : (
                        <Badge variant="outline">未認証</Badge>
                      )}
                      {node.pubkey && <span>pubkey: {node.pubkey}</span>}
                      {expiresAt && <span>有効期限: {expiresAt}</span>}
                    </div>
                    <div className="grid gap-3 md:grid-cols-2">
                      <div className="flex items-center justify-between">
                        <Label htmlFor={`community-node-role-labels-${index}`}>
                          ラベル/モデレーション
                        </Label>
                        <Switch
                          id={`community-node-role-labels-${index}`}
                          data-testid={`community-node-role-labels-${index}`}
                          checked={node.roles.labels}
                          onCheckedChange={(value) =>
                            handleRoleToggle(node.base_url, 'labels', value)
                          }
                        />
                      </div>
                      <div className="flex items-center justify-between">
                        <Label htmlFor={`community-node-role-trust-${index}`}>信頼スコア</Label>
                        <Switch
                          id={`community-node-role-trust-${index}`}
                          data-testid={`community-node-role-trust-${index}`}
                          checked={node.roles.trust}
                          onCheckedChange={(value) =>
                            handleRoleToggle(node.base_url, 'trust', value)
                          }
                        />
                      </div>
                      <div className="flex items-center justify-between">
                        <Label htmlFor={`community-node-role-search-${index}`}>検索連携</Label>
                        <Switch
                          id={`community-node-role-search-${index}`}
                          data-testid={`community-node-role-search-${index}`}
                          checked={node.roles.search}
                          onCheckedChange={(value) =>
                            handleRoleToggle(node.base_url, 'search', value)
                          }
                        />
                      </div>
                      <div className="flex items-center justify-between">
                        <Label htmlFor={`community-node-role-bootstrap-${index}`}>
                          ブートストラップ
                        </Label>
                        <Switch
                          id={`community-node-role-bootstrap-${index}`}
                          data-testid={`community-node-role-bootstrap-${index}`}
                          checked={node.roles.bootstrap}
                          onCheckedChange={(value) =>
                            handleRoleToggle(node.base_url, 'bootstrap', value)
                          }
                        />
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
          ) : (
            <p className="text-sm text-muted-foreground">登録済みのノードはありません。</p>
          )}
        </div>

        <Separator />

        <div className="space-y-3">
          <div className="flex items-center justify-between">
            <div>
              <p className="font-medium">アクセス制御 (スコープ)</p>
              <p className="text-sm text-muted-foreground">
                暗号化投稿のスコープ選択を有効化します。
              </p>
            </div>
            <Switch
              id="community-node-access-control"
              checked={enableAccessControl}
              onCheckedChange={setEnableAccessControl}
            />
          </div>
        </div>

        <Separator />

        <div className="space-y-3">
          <p className="font-medium">操作対象ノード</p>
          <Select
            value={actionNodeBaseUrl}
            onValueChange={setActionNodeBaseUrl}
            disabled={nodes.length === 0}
          >
            <SelectTrigger data-testid="community-node-action-node">
              <SelectValue placeholder="ノードを選択" />
            </SelectTrigger>
            <SelectContent>
              {nodes.map((node) => (
                <SelectItem key={node.base_url} value={node.base_url}>
                  {node.base_url}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          {!selectedNode && (
            <p className="text-xs text-muted-foreground">操作対象のノードがありません。</p>
          )}
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
            disabled={!selectedNode?.has_token}
            data-testid="community-node-accept-consents"
          >
            同意を更新
          </Button>
        </div>

        <Separator />

        <div className="space-y-3">
          <p className="font-medium">招待参加（P2P）</p>
          <p className="text-sm text-muted-foreground">
            invite.capability を貼り付けて join.request を送信します。鍵はP2P経由で届きます。
          </p>
          <Textarea
            placeholder="invite.capability のイベントJSON"
            rows={6}
            value={inviteJson}
            onChange={(e) => setInviteJson(e.target.value)}
            data-testid="community-node-invite-json"
          />
          <Button
            onClick={handleRequestJoin}
            disabled={!enableAccessControl}
            data-testid="community-node-request-join"
          >
            参加リクエストを送信
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
