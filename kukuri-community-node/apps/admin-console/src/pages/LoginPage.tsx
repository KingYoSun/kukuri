import { type FormEvent, useState } from 'react';
import { z } from 'zod';

import { Button, Card, CardContent, CardHeader, CardTitle, Input, Label, Notice } from '../components/ui';
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
      <Card className="login-panel">
        <CardHeader>
          <CardTitle>Admin Login</CardTitle>
        </CardHeader>
        <CardContent>
          <form onSubmit={submit}>
            <div className="field">
              <Label htmlFor="username">Username</Label>
              <Input
                id="username"
                value={username}
                onChange={(event) => setUsername(event.target.value)}
                autoComplete="username"
              />
            </div>
            <div className="field">
              <Label htmlFor="password">Password</Label>
              <Input
                id="password"
                type="password"
                value={password}
                onChange={(event) => setPassword(event.target.value)}
                autoComplete="current-password"
              />
            </div>
            {(localError || error) && <Notice tone="error">{localError ?? error}</Notice>}
            <Button type="submit" disabled={status === 'checking'}>
              {status === 'checking' ? 'Signing in...' : 'Sign in'}
            </Button>
          </form>
        </CardContent>
      </Card>
    </div>
  );
};
