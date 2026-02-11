import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { api } from '../lib/api';
import { AccessControlPage } from './AccessControlPage';
import { renderWithQueryClient } from '../test/renderWithQueryClient';

vi.mock('../lib/api', () => ({
  api: {
    auditLogs: vi.fn(),
    accessControlMemberships: vi.fn(),
    accessControlInvites: vi.fn(),
    accessControlDistributionResults: vi.fn(),
    issueAccessControlInvite: vi.fn(),
    revokeAccessControlInvite: vi.fn(),
    rotateAccessControl: vi.fn(),
    revokeAccessControl: vi.fn()
  }
}));

describe('AccessControlPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(api.auditLogs).mockResolvedValue([]);
    vi.mocked(api.accessControlMemberships).mockResolvedValue([
      {
        topic_id: 'kukuri:topic:test',
        scope: 'invite',
        pubkey: 'a'.repeat(64),
        status: 'active',
        joined_at: 1738809600,
        revoked_at: null,
        revoked_reason: null
      }
    ]);
    vi.mocked(api.accessControlInvites).mockResolvedValue([
      {
        topic_id: 'kukuri:topic:test',
        scope: 'invite',
        issuer_pubkey: 'f'.repeat(64),
        nonce: 'invite-nonce-1',
        event_id: 'event-1',
        expires_at: 1738896000,
        max_uses: 2,
        used_count: 0,
        status: 'active',
        revoked_at: null,
        created_at: 1738809600,
        capability_event_json: {}
      }
    ]);
    vi.mocked(api.accessControlDistributionResults).mockResolvedValue([
      {
        topic_id: 'kukuri:topic:test',
        scope: 'invite',
        epoch: 2,
        recipient_pubkey: 'a'.repeat(64),
        status: 'failed',
        reason: 'invalid pubkey',
        updated_at: 1738813200
      },
      {
        topic_id: 'kukuri:topic:test',
        scope: 'invite',
        epoch: 2,
        recipient_pubkey: 'b'.repeat(64),
        status: 'pending',
        reason: 'retry queued',
        updated_at: 1738813201
      }
    ]);
    vi.mocked(api.issueAccessControlInvite).mockResolvedValue({
      topic_id: 'kukuri:topic:test',
      scope: 'invite',
      issuer_pubkey: 'f'.repeat(64),
      nonce: 'invite-issued',
      event_id: 'event-issued',
      expires_at: 1738896000,
      max_uses: 3,
      used_count: 0,
      status: 'active',
      revoked_at: null,
      created_at: 1738809600,
      capability_event_json: {}
    });
    vi.mocked(api.revokeAccessControlInvite).mockResolvedValue({
      topic_id: 'kukuri:topic:test',
      scope: 'invite',
      issuer_pubkey: 'f'.repeat(64),
      nonce: 'invite-nonce-1',
      event_id: 'event-1',
      expires_at: 1738896000,
      max_uses: 2,
      used_count: 0,
      status: 'revoked',
      revoked_at: 1738813200,
      created_at: 1738809600,
      capability_event_json: {}
    });
    vi.mocked(api.rotateAccessControl).mockResolvedValue({
      topic_id: 'kukuri:topic:test',
      scope: 'invite',
      previous_epoch: 1,
      new_epoch: 2,
      recipients: 5,
      distribution_results: [
        {
          recipient_pubkey: 'a'.repeat(64),
          status: 'failed',
          reason: 'invalid pubkey'
        },
        {
          recipient_pubkey: 'b'.repeat(64),
          status: 'pending',
          reason: 'retry queued'
        }
      ]
    });
    vi.mocked(api.revokeAccessControl).mockResolvedValue({
      topic_id: 'kukuri:topic:test',
      scope: 'invite',
      revoked_pubkey: 'a'.repeat(64),
      previous_epoch: 2,
      new_epoch: 3,
      recipients: 4,
      distribution_results: []
    });
  });

  it('rotate と revoke の操作を送信できる', async () => {
    renderWithQueryClient(<AccessControlPage />);

    expect(await screen.findByRole('heading', { name: 'Access Control' })).toBeInTheDocument();

    const user = userEvent.setup();
    await user.type(screen.getByLabelText('Topic ID filter'), 'kukuri:topic:test');
    await user.selectOptions(screen.getByLabelText('Scope filter'), 'invite');
    await user.type(screen.getByLabelText('Pubkey filter'), 'a'.repeat(16));
    await user.click(screen.getByRole('button', { name: 'Search memberships' }));

    await waitFor(() => {
      expect(api.accessControlMemberships).toHaveBeenLastCalledWith({
        topic_id: 'kukuri:topic:test',
        scope: 'invite',
        pubkey: 'a'.repeat(16),
        status: 'active',
        limit: 200
      });
    });

    const topicInputs = screen.getAllByLabelText('Topic ID');
    await user.type(topicInputs[0], 'kukuri:topic:test');
    await user.click(screen.getByRole('button', { name: 'Rotate epoch' }));

    await waitFor(() => {
      expect(api.rotateAccessControl).toHaveBeenCalledWith({
        topic_id: 'kukuri:topic:test',
        scope: 'invite'
      });
    });

    await user.type(topicInputs[1], 'kukuri:topic:test');
    await user.type(screen.getByLabelText('Pubkey'), 'a'.repeat(64));
    await user.type(screen.getByLabelText('Reason (optional)'), 'abuse');
    await user.click(screen.getByRole('button', { name: 'Revoke + rotate' }));

    await waitFor(() => {
      expect(api.revokeAccessControl).toHaveBeenCalledWith({
        topic_id: 'kukuri:topic:test',
        scope: 'invite',
        pubkey: 'a'.repeat(64),
        reason: 'abuse'
      });
    });

    await user.type(screen.getByLabelText('Invite topic ID'), 'kukuri:topic:test');
    await user.clear(screen.getByLabelText('Expires in hours'));
    await user.type(screen.getByLabelText('Expires in hours'), '6');
    await user.clear(screen.getByLabelText('Max uses'));
    await user.type(screen.getByLabelText('Max uses'), '3');
    await user.type(screen.getByLabelText('Nonce (optional)'), 'invite-issued');
    await user.click(screen.getByRole('button', { name: 'Issue invite' }));
    await waitFor(() => {
      expect(api.issueAccessControlInvite).toHaveBeenCalledWith({
        topic_id: 'kukuri:topic:test',
        scope: 'invite',
        expires_in_seconds: 21600,
        max_uses: 3,
        nonce: 'invite-issued'
      });
    });

    await user.type(screen.getByLabelText('Invite topic filter'), 'kukuri:topic:test');
    await user.selectOptions(screen.getByLabelText('Invite status filter'), 'active');
    await user.click(screen.getByRole('button', { name: 'Search invites' }));
    await waitFor(() => {
      expect(api.accessControlInvites).toHaveBeenLastCalledWith({
        topic_id: 'kukuri:topic:test',
        status: 'active',
        limit: 200
      });
    });

    await user.type(screen.getByLabelText('Distribution topic filter'), 'kukuri:topic:test');
    await user.selectOptions(screen.getByLabelText('Distribution scope filter'), 'invite');
    await user.selectOptions(screen.getByLabelText('Distribution status filter'), 'failed');
    await user.type(screen.getByLabelText('Distribution epoch filter'), '2');
    await user.click(screen.getByRole('button', { name: 'Search results' }));
    await waitFor(() => {
      expect(api.accessControlDistributionResults).toHaveBeenLastCalledWith({
        topic_id: 'kukuri:topic:test',
        scope: 'invite',
        pubkey: undefined,
        epoch: 2,
        status: 'failed',
        limit: 200
      });
    });

    expect(screen.getAllByText('invalid pubkey').length).toBeGreaterThan(0);
    expect(screen.getAllByText('retry queued').length).toBeGreaterThan(0);

    await user.click(screen.getByRole('button', { name: 'Revoke invite' }));
    await waitFor(() => {
      expect(api.revokeAccessControlInvite).toHaveBeenCalledWith('invite-nonce-1');
    });
  });
});
