import { type FormEvent, useState } from 'react';
import { z } from 'zod';

import { errorToMessage } from '../lib/errorHandler';
import { useAuthStore } from '../store/authStore';

const loginSchema = z.object({
  username: z.string().min(1, 'Username is required'),
  password: z.string().min(1, 'Password is required')
});

export const LoginPage = () => {
  const { login, status, error } = useAuthStore();
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [localError, setLocalError] = useState<string | null>(null);

  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setLocalError(null);
    const parsed = loginSchema.safeParse({ username, password });
    if (!parsed.success) {
      setLocalError(parsed.error.issues[0]?.message ?? 'Invalid input');
      return;
    }
    try {
      await login(parsed.data.username, parsed.data.password);
      setPassword('');
    } catch (err) {
      setLocalError(errorToMessage(err));
    }
  };

  return (
    <div className="content">
      <div className="hero">
        <div>
          <h1>Admin Console</h1>
          <p>Sign in to manage services, policies, and subscriptions.</p>
        </div>
      </div>
      <div className="card login-panel">
        <h3>Admin Login</h3>
        <form onSubmit={submit}>
          <div className="field">
            <label htmlFor="username">Username</label>
            <input
              id="username"
              value={username}
              onChange={(event) => setUsername(event.target.value)}
              autoComplete="username"
            />
          </div>
          <div className="field">
            <label htmlFor="password">Password</label>
            <input
              id="password"
              type="password"
              value={password}
              onChange={(event) => setPassword(event.target.value)}
              autoComplete="current-password"
            />
          </div>
          {(localError || error) && (
            <div className="notice">{localError ?? error}</div>
          )}
          <button className="button" type="submit" disabled={status === 'checking'}>
            {status === 'checking' ? 'Signing in...' : 'Sign in'}
          </button>
        </form>
      </div>
    </div>
  );
};
