import { useEffect } from 'react';
import { Link, Outlet } from '@tanstack/react-router';
import { useQueryClient } from '@tanstack/react-query';

import { Button, Card, CardContent, CardHeader, CardTitle, Notice } from './components/ui';
import { LoginPage } from './pages/LoginPage';
import { useAuthStore } from './store/authStore';

const navItems = [
  { to: '/', label: 'Dashboard' },
  { to: '/services', label: 'Services' },
  { to: '/relay', label: 'Relay' },
  { to: '/bootstrap', label: 'Bootstrap' },
  { to: '/subscriptions', label: 'Subscriptions' },
  { to: '/policies', label: 'Policies' },
  { to: '/privacy-data', label: 'Privacy / Data' },
  { to: '/index', label: 'Index' },
  { to: '/moderation', label: 'Moderation' },
  { to: '/trust', label: 'Trust' },
  { to: '/access-control', label: 'Access Control' },
  { to: '/audit', label: 'Audit & Health' }
];

const App = () => {
  const queryClient = useQueryClient();
  const { user, status, bootstrap, logout } = useAuthStore();

  useEffect(() => {
    void bootstrap();
  }, [bootstrap]);

  const handleLogout = async () => {
    await logout();
    queryClient.clear();
  };

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
