import { useTranslation } from 'react-i18next';
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
  const { t } = useTranslation();
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
  const pendingJoinRequestsQuery = useQuery({
    queryKey: ['community-node', 'join-requests'],
    queryFn: () => accessControlApi.listJoinRequests(),
    enabled: enableAccessControl,
    staleTime: 1000 * 15,
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
        toastTitle: t('communityNodePanel.toastTrustAnchorFetchFailed'),
      });
    }
  }, [trustAnchorQuery.isError, trustAnchorQuery.error]);

  useEffect(() => {
    if (pendingJoinRequestsQuery.isError) {
      errorHandler.log('Failed to load pending join requests', pendingJoinRequestsQuery.error, {
        context: 'CommunityNodePanel.joinRequests',
        showToast: true,
        toastTitle: t('communityNodePanel.toastJoinApproveFailed'),
      });
    }
  }, [pendingJoinRequestsQuery.isError, pendingJoinRequestsQuery.error]);

  const refreshCommunityData = async () => {
    await queryClient.invalidateQueries({ queryKey: ['community-node', 'config'] });
    await queryClient.invalidateQueries({ queryKey: ['community-node', 'group-keys'] });
    await queryClient.invalidateQueries({ queryKey: ['community-node', 'consents'] });
    await queryClient.invalidateQueries({ queryKey: ['community-node', 'trust-anchor'] });
    await queryClient.invalidateQueries({ queryKey: ['community-node', 'join-requests'] });
  };

  const serializeNodes = (items: CommunityNodeConfigNodeResponse[]) =>
    items.map((item) => ({ base_url: item.base_url, roles: item.roles }));

  const handleAddNode = async () => {
    const trimmed = newBaseUrl.trim();
    if (!trimmed) {
      toast.error(t('communityNodePanel.toastEnterBaseUrl'));
      return;
    }
    const normalized = trimmed.replace(/\/+$/, '');
    if (nodes.some((node) => node.base_url === normalized)) {
      toast.error(t('communityNodePanel.toastDuplicateBaseUrl'));
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
      toast.success(t('communityNodePanel.toastAddSuccess'));
    } catch (error) {
      errorHandler.log('Community node config update failed', error, {
        context: 'CommunityNodePanel.addNode',
        showToast: true,
        toastTitle: t('communityNodePanel.toastAddFailed'),
      });
    }
  };

  const handleClearConfig = async () => {
    try {
      await communityNodeApi.clearConfig();
      await refreshCommunityData();
      setNewBaseUrl('');
      setActionNodeBaseUrl('');
      toast.success(t('communityNodePanel.toastClearSuccess'));
    } catch (error) {
      errorHandler.log('Community node config clear failed', error, {
        context: 'CommunityNodePanel.clearConfig',
        showToast: true,
        toastTitle: t('communityNodePanel.toastClearFailed'),
      });
    }
  };

  const handleRemoveNode = async (baseUrl: string) => {
    try {
      const nextNodes = nodes.filter((node) => node.base_url !== baseUrl);
      await communityNodeApi.setConfig(serializeNodes(nextNodes));
      await refreshCommunityData();
      toast.success(t('communityNodePanel.toastRemoveSuccess'));
    } catch (error) {
      errorHandler.log('Community node remove failed', error, {
        context: 'CommunityNodePanel.removeNode',
        showToast: true,
        toastTitle: t('communityNodePanel.toastRemoveFailed'),
      });
    }
  };

  const handleAuthenticate = async (baseUrl: string) => {
    try {
      await communityNodeApi.authenticate(baseUrl);
      await refreshCommunityData();
      toast.success(t('communityNodePanel.toastAuthSuccess'));
    } catch (error) {
      errorHandler.log('Community node auth failed', error, {
        context: 'CommunityNodePanel.authenticate',
        showToast: true,
        toastTitle: t('communityNodePanel.toastAuthFailed'),
      });
    }
  };

  const handleClearToken = async (baseUrl: string) => {
    try {
      await communityNodeApi.clearToken(baseUrl);
      await refreshCommunityData();
      toast.success(t('communityNodePanel.toastTokenCleared'));
    } catch (error) {
      errorHandler.log('Community node token clear failed', error, {
        context: 'CommunityNodePanel.clearToken',
        showToast: true,
        toastTitle: t('communityNodePanel.toastTokenClearFailed'),
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
        toastTitle: t('communityNodePanel.toastRoleUpdateFailed'),
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
        toastTitle: t('communityNodePanel.toastTrustAnchorUpdateFailed'),
      });
    }
  };

  const handleRequestJoin = async () => {
    if (!inviteJson.trim()) {
      toast.error(t('communityNodePanel.toastEnterInviteJson'));
      return;
    }
    try {
      const payload = JSON.parse(inviteJson);
      await accessControlApi.requestJoin({
        invite_event_json: payload,
      });
      await refreshCommunityData();
      toast.success(t('communityNodePanel.toastJoinRequestSent'));
    } catch (error) {
      errorHandler.log('Community node join request failed', error, {
        context: 'CommunityNodePanel.requestJoin',
        showToast: true,
        toastTitle: t('communityNodePanel.toastJoinRequestFailed'),
      });
    }
  };

  const handleApproveJoinRequest = async (eventId: string) => {
    try {
      await accessControlApi.approveJoinRequest({ event_id: eventId });
      await queryClient.invalidateQueries({ queryKey: ['community-node', 'join-requests'] });
      toast.success(t('communityNodePanel.toastJoinApproved'));
    } catch (error) {
      errorHandler.log('Community node join approval failed', error, {
        context: 'CommunityNodePanel.approveJoinRequest',
        showToast: true,
        toastTitle: t('communityNodePanel.toastJoinApproveFailed'),
      });
    }
  };

  const handleRejectJoinRequest = async (eventId: string) => {
    try {
      await accessControlApi.rejectJoinRequest({ event_id: eventId });
      await queryClient.invalidateQueries({ queryKey: ['community-node', 'join-requests'] });
      toast.success(t('communityNodePanel.toastJoinRejected'));
    } catch (error) {
      errorHandler.log('Community node join rejection failed', error, {
        context: 'CommunityNodePanel.rejectJoinRequest',
        showToast: true,
        toastTitle: t('communityNodePanel.toastJoinRejectFailed'),
      });
    }
  };

  const handleAcceptConsents = async () => {
    if (!selectedNode?.base_url) {
      toast.error(t('communityNodePanel.toastSelectActionNode'));
      return;
    }
    try {
      await communityNodeApi.acceptConsents({
        base_url: selectedNode.base_url,
        accept_all_current: true,
      });
      await refreshCommunityData();
      toast.success(t('communityNodePanel.toastConsentUpdated'));
    } catch (error) {
      errorHandler.log('Community node consent update failed', error, {
        context: 'CommunityNodePanel.acceptConsents',
        showToast: true,
        toastTitle: t('communityNodePanel.toastConsentUpdateFailed'),
      });
    }
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t('communityNodePanel.title')}</CardTitle>
        <CardDescription>{t('communityNodePanel.description')}</CardDescription>
      </CardHeader>
      <CardContent className="space-y-6">
        <div className="space-y-3">
          <Label htmlFor="community-node-base-url">{t('communityNodePanel.baseUrl')}</Label>
          <Input
            id="community-node-base-url"
            data-testid="community-node-base-url"
            placeholder={t('communityNodePanel.baseUrlPlaceholder')}
            value={newBaseUrl}
            onChange={(e) => setNewBaseUrl(e.target.value)}
          />
          <div className="flex flex-wrap items-center gap-2">
            <Button
              variant="outline"
              onClick={handleAddNode}
              data-testid="community-node-save-config"
            >
              {t('communityNodePanel.add')}
            </Button>
            <Button
              variant="ghost"
              onClick={handleClearConfig}
              data-testid="community-node-clear-config"
              disabled={nodes.length === 0}
            >
              {t('communityNodePanel.clearAll')}
            </Button>
          </div>
        </div>

        <Separator />

        <div className="space-y-3">
          <div>
            <p className="font-medium">{t('communityNodePanel.trustAnchor')}</p>
            <p className="text-sm text-muted-foreground">{t('communityNodePanel.trustAnchorHint')}</p>
          </div>
          <Select
            value={trustAnchorAttester}
            onValueChange={handleTrustAnchorChange}
            disabled={trustAttesterOptions.length === 0}
          >
            <SelectTrigger data-testid="community-node-trust-anchor">
              <SelectValue placeholder={t('communityNodePanel.auto')} />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="auto">{t('communityNodePanel.autoMode')}</SelectItem>
              {trustAttesterOptions.map((option) => (
                <SelectItem key={option.value} value={option.value}>
                  {option.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          {trustAnchorAttester === 'auto' ? (
            <p className="text-xs text-muted-foreground">
              {t('communityNodePanel.autoModeHint')}
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
              {t('communityNodePanel.attesterNotFound')}
            </p>
          )}
        </div>

        <Separator />

        <div className="space-y-3">
          <div className="flex items-center justify-between">
            <div>
              <p className="font-medium">{t('communityNodePanel.adoptedNodes')}</p>
              <p className="text-sm text-muted-foreground">
                {t('communityNodePanel.adoptedNodesHint')}
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
                          {t('communityNodePanel.authenticate')}
                        </Button>
                        <Button
                          size="sm"
                          variant="ghost"
                          onClick={() => handleClearToken(node.base_url)}
                          disabled={!node.has_token}
                          data-testid={`community-node-clear-token-${index}`}
                        >
                          {t('communityNodePanel.clearToken')}
                        </Button>
                        <Button
                          size="sm"
                          variant="ghost"
                          onClick={() => handleRemoveNode(node.base_url)}
                          data-testid={`community-node-remove-${index}`}
                        >
                          {t('communityNodePanel.remove')}
                        </Button>
                      </div>
                    </div>
                    <div
                      className="flex flex-wrap items-center gap-2 text-xs text-muted-foreground"
                      data-testid={`community-node-token-status-${index}`}
                      data-has-token={node.has_token ? 'true' : 'false'}
                      data-pubkey={node.pubkey ?? ''}
                    >
                      <span>{t('communityNodePanel.authStatus')}</span>
                      {node.has_token ? (
                        <Badge variant="secondary">{t('communityNodePanel.valid')}</Badge>
                      ) : (
                        <Badge variant="outline">{t('communityNodePanel.unauthenticated')}</Badge>
                      )}
                      {node.pubkey && <span>pubkey: {node.pubkey}</span>}
                      {expiresAt && <span>{t('communityNodePanel.expiresAt')} {expiresAt}</span>}
                    </div>
                    <div className="grid gap-3 md:grid-cols-2">
                      <div className="flex items-center justify-between">
                        <Label htmlFor={`community-node-role-labels-${index}`}>
                          {t('communityNodePanel.roleLabels')}
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
                        <Label htmlFor={`community-node-role-trust-${index}`}>{t('communityNodePanel.roleTrust')}</Label>
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
                        <Label htmlFor={`community-node-role-search-${index}`}>{t('communityNodePanel.roleSearch')}</Label>
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
                          {t('communityNodePanel.bootstrap')}
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
            <p className="text-sm text-muted-foreground">{t('communityNodePanel.noNodesRegistered')}</p>
          )}
        </div>

        <Separator />

        <div className="space-y-3">
          <div className="flex items-center justify-between">
            <div>
              <p className="font-medium">{t('communityNodePanel.accessControl')}</p>
              <p className="text-sm text-muted-foreground">
                {t('communityNodePanel.accessControlHint')}
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
          <p className="font-medium">{t('communityNodePanel.actionNode')}</p>
          <Select
            value={actionNodeBaseUrl}
            onValueChange={setActionNodeBaseUrl}
            disabled={nodes.length === 0}
          >
            <SelectTrigger data-testid="community-node-action-node">
              <SelectValue placeholder={t('communityNodePanel.selectNode')} />
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
            <p className="text-xs text-muted-foreground">{t('communityNodePanel.noActionNode')}</p>
          )}
        </div>

        <Separator />

        <div className="space-y-3">
          <p className="font-medium">{t('communityNodePanel.consentStatus')}</p>
          <div
            className="rounded-md border p-3 text-xs text-muted-foreground"
            data-testid="community-node-consents"
          >
            {consentsQuery.data ? JSON.stringify(consentsQuery.data, null, 2) : t('communityNodePanel.notFetched')}
          </div>
          <Button
            variant="outline"
            onClick={handleAcceptConsents}
            disabled={!selectedNode?.has_token}
            data-testid="community-node-accept-consents"
          >
            {t('communityNodePanel.updateConsent')}
          </Button>
        </div>

        <Separator />

        <div className="space-y-3">
          <p className="font-medium">{t('communityNodePanel.inviteP2PTitle')}</p>
          <p className="text-sm text-muted-foreground">
            {t('communityNodePanel.inviteP2PHint')}
          </p>
          <Textarea
            placeholder={t('communityNodePanel.invitePlaceholder')}
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
            {t('communityNodePanel.sendJoinRequest')}
          </Button>
        </div>

        <Separator />

        <div className="space-y-3">
          <p className="font-medium">{t('communityNodePanel.joinRequestInbox')}</p>
          <p className="text-sm text-muted-foreground">
            {t('communityNodePanel.joinRequestInboxHint')}
          </p>
          {pendingJoinRequestsQuery.data?.items &&
          pendingJoinRequestsQuery.data.items.length > 0 ? (
            <div className="space-y-2" data-testid="community-node-join-requests">
              {pendingJoinRequestsQuery.data.items.map((item) => {
                const requestedAt = item.requested_at
                  ? new Date(item.requested_at * 1000).toLocaleString()
                  : null;
                const receivedAt = new Date(item.received_at * 1000).toLocaleString();
                return (
                  <div key={item.event_id} className="rounded-md border p-3 text-sm space-y-2">
                    <div className="flex flex-wrap items-center justify-between gap-2">
                      <div className="font-medium break-all">{item.topic_id}</div>
                      <Badge variant="outline">{item.scope}</Badge>
                    </div>
                    <div className="text-xs text-muted-foreground">
                      requester: {shortenPubkey(item.requester_pubkey)} / received: {receivedAt}
                      {requestedAt ? ` / requested: ${requestedAt}` : ''}
                    </div>
                    <div className="flex flex-wrap gap-2">
                      <Button
                        size="sm"
                        onClick={() => handleApproveJoinRequest(item.event_id)}
                        disabled={!enableAccessControl}
                        data-testid={`community-node-join-approve-${item.event_id}`}
                      >
                        {t('communityNodePanel.approveAndDistribute')}
                      </Button>
                      <Button
                        size="sm"
                        variant="ghost"
                        onClick={() => handleRejectJoinRequest(item.event_id)}
                        disabled={!enableAccessControl}
                        data-testid={`community-node-join-reject-${item.event_id}`}
                      >
                        {t('communityNodePanel.reject')}
                      </Button>
                    </div>
                  </div>
                );
              })}
            </div>
          ) : (
            <p className="text-sm text-muted-foreground">{t('communityNodePanel.noPendingJoinRequest')}</p>
          )}
        </div>
        <Separator />

        <div className="space-y-3">
          <p className="font-medium">{t('communityNodePanel.savedKeys')}</p>
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
            <p className="text-sm text-muted-foreground">{t('communityNodePanel.noSavedKeys')}</p>
          )}
        </div>
      </CardContent>
    </Card>
  );
}
