import { screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { api } from '../lib/api';
import { renderWithQueryClient } from '../test/renderWithQueryClient';
import { BootstrapPage } from './BootstrapPage';

vi.mock('../lib/api', () => ({
  api: {
    nodeSubscriptions: vi.fn(),
    subscriptions: vi.fn()
  }
}));

describe('BootstrapPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(api.nodeSubscriptions).mockResolvedValue([]);
    vi.mocked(api.subscriptions).mockResolvedValue([]);
  });

  it('接続先を node_id@host:port 表記で表示し、接続ユーザー数と一覧を同期する', async () => {
    vi.mocked(api.nodeSubscriptions).mockResolvedValue([
      {
        topic_id: 'topic-alpha',
        enabled: true,
        ref_count: 2,
        ingest_policy: null,
        connected_nodes: ['node-a@bootstrap.example:11233', 'node-b:3344'],
        connected_node_count: 2,
        connected_users: ['pubkey-active-1', 'pubkey-active-2'],
        connected_user_count: 2,
        updated_at: 1700000000
      },
      {
        topic_id: 'topic-beta',
        enabled: true,
        ref_count: 1,
        ingest_policy: null,
        connected_nodes: ['node-a@bootstrap.example:11233'],
        connected_node_count: 1,
        connected_users: ['pubkey-active-2'],
        connected_user_count: 1,
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

    renderWithQueryClient(<BootstrapPage />);

    expect(await screen.findByRole('heading', { name: 'Bootstrap' })).toBeInTheDocument();
    expect(await screen.findByText('Connected users: 2')).toBeInTheDocument();
    expect(await screen.findByText('node-a@bootstrap.example:11233')).toBeInTheDocument();
    expect(await screen.findByText('node-b:3344@unknown:0')).toBeInTheDocument();
    expect(await screen.findByText('pubkey-active-1')).toBeInTheDocument();
    expect(await screen.findByText('pubkey-active-2')).toBeInTheDocument();
    expect(screen.queryByText('pubkey-latest-paused')).not.toBeInTheDocument();
    expect(screen.getByText('node_id@host:port')).toBeInTheDocument();
  });

  it('接続ユーザー0件時の空表示を出す', async () => {
    renderWithQueryClient(<BootstrapPage />);

    expect(await screen.findByText('Connected users: 0')).toBeInTheDocument();
    expect(await screen.findByText('No connected nodes')).toBeInTheDocument();
    expect(await screen.findByText('No connected users')).toBeInTheDocument();
  });
});
