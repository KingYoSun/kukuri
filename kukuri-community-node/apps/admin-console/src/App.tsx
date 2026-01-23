import { useEffect } from 'react';
import { Link, Outlet } from '@tanstack/react-router';
import { useQueryClient } from '@tanstack/react-query';

import { LoginPage } from './pages/LoginPage';
import { useAuthStore } from './store/authStore';

const navItems = [
  { to: '/', label: 'Dashboard' },
  { to: '/services', label: 'Services' },
  { to: '/subscriptions', label: 'Subscriptions' },
  { to: '/policies', label: 'Policies' },
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
          <p className="notice">Community Node Admin</p>
        </div>
        <nav className="nav">
          {navItems.map((item) => (
            <Link key={item.to} to={item.to} activeProps={{ className: 'active' }}>
              {item.label}
            </Link>
          ))}
        </nav>
        <div className="card">
          <h3>Session</h3>
          <p>{user.username}</p>
          <button className="button secondary" onClick={handleLogout}>
            Sign out
          </button>
        </div>
      </aside>
      <main className="content">
        <Outlet />
      </main>
    </div>
  );
};

export default App;
