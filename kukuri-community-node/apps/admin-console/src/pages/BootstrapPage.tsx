import { useMemo } from 'react';
import { useQuery } from '@tanstack/react-query';

import { Card, CardContent, CardHeader, CardTitle, Notice } from '../components/ui';
import { api } from '../lib/api';
import { normalizeConnectedNode } from '../lib/bootstrap';
import { errorToMessage } from '../lib/errorHandler';
import { subscriptionsQueryOptions } from '../lib/subscriptionsQuery';
import type { NodeSubscription, SubscriptionRow } from '../lib/types';

const collectConnectedUsers = (rows: SubscriptionRow[]): string[] => {
  const latestByUser = new Map<string, SubscriptionRow>();
  for (const row of rows) {
    const pubkey = row.subscriber_pubkey.trim();
    if (pubkey === '') {
      continue;
    }
    const current = latestByUser.get(pubkey);
    if (!current || row.started_at > current.started_at) {
      latestByUser.set(pubkey, row);
    }
  }

  return Array.from(latestByUser.values())
    .filter((row) => row.status === 'active')
    .map((row) => row.subscriber_pubkey.trim())
    .sort();
};

export const BootstrapPage = () => {
  const nodeSubscriptionsQuery = useQuery<NodeSubscription[]>({
    queryKey: ['nodeSubscriptions'],
    queryFn: api.nodeSubscriptions
  });
  const subscriptionsQuery = useQuery<SubscriptionRow[]>(subscriptionsQueryOptions(''));

  const connectedNodes = useMemo(() => {
    const rawNodes = (nodeSubscriptionsQuery.data ?? []).flatMap(
      (subscription) => subscription.connected_nodes ?? []
    );
    return Array.from(new Set(rawNodes.map(normalizeConnectedNode))).sort();
  }, [nodeSubscriptionsQuery.data]);

  const connectedUsers = useMemo(
    () => collectConnectedUsers(subscriptionsQuery.data ?? []),
    [subscriptionsQuery.data]
  );

  const bootstrapError = [nodeSubscriptionsQuery.error, subscriptionsQuery.error]
    .map((err) => (err ? errorToMessage(err) : null))
    .find((message): message is string => message !== null);
  const isBootstrapLoading = nodeSubscriptionsQuery.isLoading || subscriptionsQuery.isLoading;

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
            <div className="muted">Connected users: {connectedUsers.length}</div>
            <div className="muted">Connected nodes: {connectedNodes.length}</div>
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
            {connectedNodes.length === 0 ? (
              <div className="muted">No connected nodes</div>
            ) : (
              <div className="stack">
                {connectedNodes.map((node) => (
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
              <div className="muted">No connected users</div>
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
