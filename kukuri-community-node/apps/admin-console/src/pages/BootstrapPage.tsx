import { useMemo } from 'react';
import { useQuery } from '@tanstack/react-query';

import { Card, CardContent, CardHeader, CardTitle, Notice } from '../components/ui';
import { api } from '../lib/api';
import { normalizeConnectedNode } from '../lib/bootstrap';
import { errorToMessage } from '../lib/errorHandler';
import { parseRelayRuntimeSnapshot } from '../lib/relayRuntime';
import type { NodeSubscription, ServiceInfo } from '../lib/types';

export const BootstrapPage = () => {
  const nodeSubscriptionsQuery = useQuery<NodeSubscription[]>({
    queryKey: ['nodeSubscriptions'],
    queryFn: api.nodeSubscriptions
  });
  const servicesQuery = useQuery<ServiceInfo[]>({
    queryKey: ['services'],
    queryFn: api.services,
    refetchInterval: 5000
  });

  const connectedNodes = useMemo(() => {
    const rawNodes = (nodeSubscriptionsQuery.data ?? []).flatMap(
      (subscription) => subscription.connected_nodes ?? []
    );
    return Array.from(new Set(rawNodes.map(normalizeConnectedNode))).sort();
  }, [nodeSubscriptionsQuery.data]);

  const connectedUsers = useMemo(() => {
    const uniqueUsers = new Set<string>();
    for (const subscription of nodeSubscriptionsQuery.data ?? []) {
      for (const user of subscription.connected_users ?? []) {
        const trimmed = user.trim();
        if (trimmed !== '') {
          uniqueUsers.add(trimmed);
        }
      }
    }
    return Array.from(uniqueUsers).sort();
  }, [nodeSubscriptionsQuery.data]);

  const connectedUserCountFromTopics = useMemo(() => {
    return (nodeSubscriptionsQuery.data ?? []).reduce((total, subscription) => {
      if (
        typeof subscription.connected_user_count === 'number' &&
        Number.isFinite(subscription.connected_user_count)
      ) {
        return total + subscription.connected_user_count;
      }
      return total + (subscription.connected_users ?? []).length;
    }, 0);
  }, [nodeSubscriptionsQuery.data]);

  const relayRuntime = useMemo(
    () => parseRelayRuntimeSnapshot(servicesQuery.data),
    [servicesQuery.data]
  );
  const summaryConnectedNodes = connectedNodes.length > 0 ? connectedNodes : relayRuntime.bootstrapNodes;
  const runtimeConnectedUsers = relayRuntime.wsConnections ?? 0;
  const summaryConnectedUsers =
    connectedUsers.length > 0
      ? connectedUsers.length
      : connectedUserCountFromTopics > 0
        ? connectedUserCountFromTopics
        : runtimeConnectedUsers;
  const usesRuntimeNodeFallback = connectedNodes.length === 0 && summaryConnectedNodes.length > 0;
  const usesRuntimeUserFallback =
    connectedUsers.length === 0 && connectedUserCountFromTopics === 0 && runtimeConnectedUsers > 0;
  const usersCountWithoutPubkeys = connectedUsers.length === 0 && summaryConnectedUsers > 0;

  const bootstrapError = [nodeSubscriptionsQuery.error, servicesQuery.error]
    .map((err) => (err ? errorToMessage(err) : null))
    .find((message): message is string => message !== null);
  const isBootstrapLoading = nodeSubscriptionsQuery.isLoading || servicesQuery.isLoading;

  return (
    <>
      <div className="hero">
        <div>
          <h1>Bootstrap</h1>
          <p>Connection endpoints and active users for bootstrap routing.</p>
        </div>
      </div>
      <div className="grid">
        <Card>
          <CardHeader>
            <CardTitle>Summary</CardTitle>
          </CardHeader>
          <CardContent className="stack">
            <div className="muted">Connected users: {summaryConnectedUsers}</div>
            <div className="muted">Connected nodes: {summaryConnectedNodes.length}</div>
            {usesRuntimeNodeFallback && (
              <div className="muted">Connected nodes are sourced from relay runtime p2p info.</div>
            )}
            {usesRuntimeUserFallback && (
              <div className="muted">Connected users are sourced from relay runtime ws connections.</div>
            )}
          </CardContent>
        </Card>
        <Card>
          <CardHeader>
            <CardTitle>Node format</CardTitle>
          </CardHeader>
          <CardContent>
            <code>node_id@host:port</code>
          </CardContent>
        </Card>
      </div>
      <div className="grid">
        <Card>
          <CardHeader>
            <CardTitle>Connected nodes</CardTitle>
          </CardHeader>
          <CardContent>
            {summaryConnectedNodes.length === 0 ? (
              <div className="muted">No connected nodes</div>
            ) : (
              <div className="stack">
                {summaryConnectedNodes.map((node) => (
                  <code key={node}>{node}</code>
                ))}
              </div>
            )}
          </CardContent>
        </Card>
        <Card>
          <CardHeader>
            <CardTitle>Users</CardTitle>
          </CardHeader>
          <CardContent>
            {connectedUsers.length === 0 ? (
              usersCountWithoutPubkeys ? (
                <div className="muted">
                  Pubkeys are unavailable. Relay runtime reports {summaryConnectedUsers}{' '}
                  websocket connection(s).
                </div>
              ) : (
                <div className="muted">No connected users</div>
              )
            ) : (
              <div className="stack">
                {connectedUsers.map((userPubkey) => (
                  <code key={userPubkey}>{userPubkey}</code>
                ))}
              </div>
            )}
          </CardContent>
        </Card>
      </div>
      {isBootstrapLoading && <Notice>Loading bootstrap data...</Notice>}
      {bootstrapError && <Notice tone="error">{bootstrapError}</Notice>}
    </>
  );
};
