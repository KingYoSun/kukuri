import { useEffect, useMemo } from 'react';
import { Link, Outlet } from '@tanstack/react-router';
import { useQuery, useQueryClient } from '@tanstack/react-query';

import { Button, Card, CardContent, CardHeader, CardTitle, Notice } from './components/ui';
import { api } from './lib/api';
import { normalizeConnectedNode } from './lib/bootstrap';
import { errorToMessage } from './lib/errorHandler';
import { subscriptionsQueryOptions } from './lib/subscriptionsQuery';
import type { NodeSubscription, SubscriptionRow } from './lib/types';
import { LoginPage } from './pages/LoginPage';
import { useAuthStore } from './store/authStore';

const navItems = [
  { to: '/', label: 'Dashboard' },
  { to: '/services', label: 'Services' },
  { to: '/relay', label: 'Relay' },
  { to: '/subscriptions', label: 'Subscriptions' },
  { to: '/policies', label: 'Policies' },
  { to: '/privacy-data', label: 'Privacy / Data' },
  { to: '/index', label: 'Index' },
  { to: '/moderation', label: 'Moderation' },
  { to: '/trust', label: 'Trust' },
  { to: '/access-control', label: 'Access Control' },
  { to: '/audit', label: 'Audit & Health' }
];

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

const App = () => {
  const queryClient = useQueryClient();
  const { user, status, bootstrap, logout } = useAuthStore();

  const nodeSubscriptionsQuery = useQuery<NodeSubscription[]>({
    queryKey: ['nodeSubscriptions'],
    queryFn: api.nodeSubscriptions,
    enabled: Boolean(user)
  });
  const subscriptionsQuery = useQuery<SubscriptionRow[]>({
    ...subscriptionsQueryOptions(''),
    enabled: Boolean(user)
  });

  useEffect(() => {
    void bootstrap();
  }, [bootstrap]);

  const handleLogout = async () => {
    await logout();
    queryClient.clear();
  };

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

  if (status === 'unknown' || (status === 'checking' && !user)) {
    return (
      <div className="content">
        <div className="hero">
          <div>
            <h1>Admin Console</h1>
            <p>Checking session...</p>
          </div>
        </div>
      </div>
    );
  }

  if (!user) {
    return <LoginPage />;
  }

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div>
          <div className="brand">Kukuri Node</div>
          <Notice>Community Node Admin</Notice>
        </div>
        <nav className="nav">
          {navItems.map((item) => (
            <Link key={item.to} to={item.to} activeProps={{ className: 'active' }}>
              {item.label}
            </Link>
          ))}
        </nav>
        <Card>
          <CardHeader>
            <CardTitle>Bootstrap</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="muted">Connected users: {connectedUsers.length}</div>
            <div className="stack">
              <div>
                <div className="muted">Connected nodes</div>
                {connectedNodes.length === 0 ? (
                  <div className="muted">No connected nodes</div>
                ) : (
                  <div className="stack">
                    {connectedNodes.map((node) => (
                      <code key={node}>{node}</code>
                    ))}
                  </div>
                )}
              </div>
              <div>
                <div className="muted">Users</div>
                {connectedUsers.length === 0 ? (
                  <div className="muted">No connected users</div>
                ) : (
                  <div className="stack">
                    {connectedUsers.map((userPubkey) => (
                      <code key={userPubkey}>{userPubkey}</code>
                    ))}
                  </div>
                )}
              </div>
            </div>
            {isBootstrapLoading && <Notice>Loading bootstrap data...</Notice>}
            {bootstrapError && <Notice tone="error">{bootstrapError}</Notice>}
          </CardContent>
        </Card>
        <Card>
          <CardHeader>
            <CardTitle>Session</CardTitle>
          </CardHeader>
          <CardContent>
            <p>{user.username}</p>
            <Button variant="secondary" onClick={handleLogout}>
              Sign out
            </Button>
          </CardContent>
        </Card>
      </aside>
      <main className="content">
        <Outlet />
      </main>
    </div>
  );
};

export default App;
