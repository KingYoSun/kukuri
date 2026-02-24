import { fireEvent, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { api } from '../lib/api';
import { RelayPage } from './RelayPage';
import { renderWithQueryClient } from '../test/renderWithQueryClient';

vi.mock('../lib/api', () => ({
  api: {
    nodeSubscriptions: vi.fn(),
    createNodeSubscription: vi.fn(),
    updateNodeSubscription: vi.fn(),
    deleteNodeSubscription: vi.fn()
  }
}));

describe('RelayPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(api.nodeSubscriptions).mockResolvedValue([
      {
        topic_id: 'kukuri:topic:1',
        enabled: true,
        ref_count: 2,
        ingest_policy: {
          retention_days: 7,
          max_events: 100,
          max_bytes: 2048,
          allow_backfill: true
        },
        connected_nodes: ['node-1@relay.example.com:7777'],
        connected_node_count: 1,
        connected_users: ['a'.repeat(64), 'b'.repeat(64)],
        connected_user_count: 2,
        updated_at: 1738809601
      }
    ]);
    vi.mocked(api.createNodeSubscription).mockResolvedValue({
      topic_id: 'kukuri:topic:new',
      enabled: true,
      ref_count: 0,
      ingest_policy: {
        retention_days: 30,
        max_events: 500,
        max_bytes: 8192,
        allow_backfill: true
      },
      connected_nodes: [],
      connected_node_count: 0,
      connected_users: [],
      connected_user_count: 0,
      updated_at: 1738809602
    });
    vi.mocked(api.updateNodeSubscription).mockResolvedValue({
      topic_id: 'kukuri:topic:1',
      enabled: false,
      ref_count: 1,
      ingest_policy: {
        retention_days: 7,
        max_events: 100,
        max_bytes: 2048,
        allow_backfill: true
      },
      connected_nodes: ['node-1@relay.example.com:7777'],
      connected_node_count: 1,
      connected_users: ['a'.repeat(64), 'b'.repeat(64)],
      connected_user_count: 2,
      updated_at: 1738809602
    });
    vi.mocked(api.deleteNodeSubscription).mockResolvedValue({
      status: 'deleted'
    });
  });

  it('トピック購読CRUDと接続ユーザー表示が機能する', async () => {
    renderWithQueryClient(<RelayPage />);

    expect(await screen.findByRole('heading', { name: 'Relay', level: 1 })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Topic Subscriptions' })).toBeInTheDocument();
    expect(screen.getByRole('columnheader', { name: 'Connected Users' })).toBeInTheDocument();
    expect(screen.getByRole('columnheader', { name: 'User Pubkeys' })).toBeInTheDocument();
    expect(await screen.findByText('node-1@relay.example.com:7777')).toBeInTheDocument();
    expect(
      await screen.findByText((content) => content.includes('Connected users:') && content.includes('2'))
    ).toBeInTheDocument();
    expect(await screen.findByText('a'.repeat(64))).toBeInTheDocument();
    expect(await screen.findByText('b'.repeat(64))).toBeInTheDocument();

    const user = userEvent.setup();

    await user.click(screen.getByRole('button', { name: 'Toggle' }));
    await waitFor(() => {
      expect(api.updateNodeSubscription).toHaveBeenCalledWith('kukuri:topic:1', {
        enabled: false,
        ingest_policy: {
          retention_days: 7,
          max_events: 100,
          max_bytes: 2048,
          allow_backfill: true
        }
      });
    });

    fireEvent.change(screen.getByLabelText('Retention days kukuri:topic:1'), {
      target: { value: '14' }
    });
    fireEvent.change(screen.getByLabelText('Max events kukuri:topic:1'), {
      target: { value: '200' }
    });
    fireEvent.change(screen.getByLabelText('Max bytes kukuri:topic:1'), {
      target: { value: '4096' }
    });
    fireEvent.change(screen.getByLabelText('Backfill kukuri:topic:1'), {
      target: { value: 'disabled' }
    });
    await user.click(screen.getByRole('button', { name: 'Save policy' }));
    await waitFor(() => {
      expect(api.updateNodeSubscription).toHaveBeenLastCalledWith('kukuri:topic:1', {
        enabled: true,
        ingest_policy: {
          retention_days: 14,
          max_events: 200,
          max_bytes: 4096,
          allow_backfill: false
        }
      });
    });

    fireEvent.change(screen.getByLabelText('New topic ID'), {
      target: { value: 'kukuri:topic:new' }
    });
    fireEvent.change(screen.getByLabelText('New retention days'), {
      target: { value: '30' }
    });
    fireEvent.change(screen.getByLabelText('New max events'), {
      target: { value: '500' }
    });
    fireEvent.change(screen.getByLabelText('New max bytes'), {
      target: { value: '8192' }
    });
    await user.click(screen.getByRole('button', { name: 'Add topic subscription' }));
    await waitFor(() => {
      expect(api.createNodeSubscription).toHaveBeenCalledWith({
        topic_id: 'kukuri:topic:new',
        enabled: true,
        ingest_policy: {
          retention_days: 30,
          max_events: 500,
          max_bytes: 8192,
          allow_backfill: true
        }
      });
    });

    await user.click(screen.getByRole('button', { name: 'Delete topic' }));
    await waitFor(() => {
      expect(api.deleteNodeSubscription).toHaveBeenCalledWith('kukuri:topic:1');
    });
  });
});
