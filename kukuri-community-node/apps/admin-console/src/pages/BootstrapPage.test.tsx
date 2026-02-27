import { screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { api } from '../lib/api';
import { renderWithQueryClient } from '../test/renderWithQueryClient';
import { BootstrapPage } from './BootstrapPage';

vi.mock('../lib/api', () => ({
  api: {
    nodeSubscriptions: vi.fn(),
    subscriptions: vi.fn(),
    services: vi.fn()
  }
}));

describe('BootstrapPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(api.nodeSubscriptions).mockResolvedValue([]);
    vi.mocked(api.subscriptions).mockResolvedValue([]);
    vi.mocked(api.services).mockResolvedValue([]);
  });

  it('shows normalized nodes and active user summary', async () => {
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

  it('shows empty state when both subscription and runtime data are unavailable', async () => {
    renderWithQueryClient(<BootstrapPage />);

    expect(await screen.findByText('Connected users: 0')).toBeInTheDocument();
    expect(await screen.findByText('No connected nodes')).toBeInTheDocument();
    expect(await screen.findByText('No connected users')).toBeInTheDocument();
  });

  it('falls back to relay runtime data when topic subscription connectivity is empty', async () => {
    vi.mocked(api.services).mockResolvedValue([
      {
        service: 'relay',
        version: 3,
        config_json: {},
        updated_at: 1700001000,
        updated_by: 'test-admin',
        health: {
          status: 'healthy',
          checked_at: 1700001000,
          details: {
            auth_transition: {
              ws_connections: 3
            },
            p2p_runtime: {
              bootstrap_nodes: ['relay-node@127.0.0.1:7777']
            }
          }
        }
      }
    ]);

    renderWithQueryClient(<BootstrapPage />);

    expect(await screen.findByText('Connected users: 3')).toBeInTheDocument();
    expect(await screen.findByText('Connected nodes: 1')).toBeInTheDocument();
    expect(await screen.findByText('relay-node@127.0.0.1:7777')).toBeInTheDocument();
    expect(
      await screen.findByText('Pubkeys are unavailable. Relay runtime reports 3 websocket connection(s).')
    ).toBeInTheDocument();
  });
});
