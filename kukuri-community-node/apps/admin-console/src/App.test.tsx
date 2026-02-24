import type { ReactNode } from 'react';
import { useQuery } from '@tanstack/react-query';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import App from './App';
import { api } from './lib/api';
import { subscriptionsQueryOptions } from './lib/subscriptionsQuery';
import { useAuthStore } from './store/authStore';
import { renderWithQueryClient } from './test/renderWithQueryClient';

vi.mock('./lib/api', () => ({
  api: {
    me: vi.fn(),
    login: vi.fn(),
    logout: vi.fn(),
    nodeSubscriptions: vi.fn(),
    subscriptions: vi.fn()
  }
}));

let outletContent: ReactNode = null;

vi.mock('@tanstack/react-router', () => ({
  Link: ({ children }: { children: ReactNode }) => <span>{children}</span>,
  Outlet: () => outletContent ?? <div data-testid="app-outlet" />
}));

const adminUser = {
  admin_user_id: 'admin-1',
  username: 'admin'
};

describe('App auth flow', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    outletContent = null;
    useAuthStore.setState({ user: null, status: 'unknown', error: undefined });
    vi.mocked(api.logout).mockResolvedValue({ status: 'ok' });
    vi.mocked(api.nodeSubscriptions).mockResolvedValue([]);
    vi.mocked(api.subscriptions).mockResolvedValue([]);
  });

  it('セッションブートストラップで未認証時はログイン画面を表示する', async () => {
    vi.mocked(api.me).mockRejectedValue(Object.assign(new Error('Unauthorized'), { status: 401 }));

    renderWithQueryClient(<App />);

    expect(screen.getByText('Checking session...')).toBeInTheDocument();
    expect(await screen.findByRole('heading', { name: 'Admin Login' })).toBeInTheDocument();
    expect(api.me).toHaveBeenCalledTimes(1);
  });

  it('セッションが有効な場合はサインアウト後にログイン画面へ遷移する', async () => {
    vi.mocked(api.me).mockResolvedValue(adminUser);

    renderWithQueryClient(<App />);

    expect(await screen.findByText('admin')).toBeInTheDocument();

    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: 'Sign out' }));

    await waitFor(() => {
      expect(api.logout).toHaveBeenCalledTimes(1);
    });
    expect(await screen.findByRole('heading', { name: 'Admin Login' })).toBeInTheDocument();
  });

  it('Bootstrapカードに接続先と接続ユーザー一覧を表示する', async () => {
    vi.mocked(api.me).mockResolvedValue(adminUser);
    vi.mocked(api.nodeSubscriptions).mockResolvedValue([
      {
        topic_id: 'topic-alpha',
        enabled: true,
        ref_count: 2,
        ingest_policy: null,
        connected_nodes: ['node-a@bootstrap.example:11233', 'node-b:3344'],
        connected_node_count: 2,
        updated_at: 1700000000
      },
      {
        topic_id: 'topic-beta',
        enabled: true,
        ref_count: 1,
        ingest_policy: null,
        connected_nodes: ['node-a@bootstrap.example:11233'],
        connected_node_count: 1,
        updated_at: 1700000100
      }
    ]);
    vi.mocked(api.subscriptions).mockResolvedValue([
      {
        subscription_id: 'sub-1',
        subscriber_pubkey: 'pubkey-active-1',
        plan_id: 'basic',
        status: 'active',
        started_at: 200,
        ended_at: null
      },
      {
        subscription_id: 'sub-2',
        subscriber_pubkey: 'pubkey-latest-paused',
        plan_id: 'basic',
        status: 'active',
        started_at: 100,
        ended_at: null
      },
      {
        subscription_id: 'sub-3',
        subscriber_pubkey: 'pubkey-latest-paused',
        plan_id: 'basic',
        status: 'paused',
        started_at: 300,
        ended_at: 320
      },
      {
        subscription_id: 'sub-4',
        subscriber_pubkey: 'pubkey-active-2',
        plan_id: 'pro',
        status: 'active',
        started_at: 400,
        ended_at: null
      }
    ]);

    renderWithQueryClient(<App />);

    expect(await screen.findByRole('heading', { name: 'Bootstrap' })).toBeInTheDocument();
    expect(await screen.findByText('Connected users: 2')).toBeInTheDocument();
    expect(await screen.findByText('node-a@bootstrap.example:11233')).toBeInTheDocument();
    expect(await screen.findByText('node-b:3344@unknown:0')).toBeInTheDocument();
    expect(await screen.findByText('pubkey-active-1')).toBeInTheDocument();
    expect(await screen.findByText('pubkey-active-2')).toBeInTheDocument();
    expect(screen.queryByText('pubkey-latest-paused')).not.toBeInTheDocument();
  });

  it('サイドバーと購読ページのクエリキーを共有して購読APIの重複呼び出しを防ぐ', async () => {
    vi.mocked(api.me).mockResolvedValue(adminUser);

    const SubscriptionsOutlet = () => {
      useQuery(subscriptionsQueryOptions(''));
      return <div data-testid="subscriptions-outlet" />;
    };
    outletContent = <SubscriptionsOutlet />;

    renderWithQueryClient(<App />);

    expect(await screen.findByText('admin')).toBeInTheDocument();
    await waitFor(() => {
      expect(api.subscriptions).toHaveBeenCalledTimes(1);
    });
  });
});
