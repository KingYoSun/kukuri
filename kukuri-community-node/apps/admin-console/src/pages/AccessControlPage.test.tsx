import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { api } from '../lib/api';
import { AccessControlPage } from './AccessControlPage';
import { renderWithQueryClient } from '../test/renderWithQueryClient';

vi.mock('../lib/api', () => ({
  api: {
    auditLogs: vi.fn(),
    rotateAccessControl: vi.fn(),
    revokeAccessControl: vi.fn()
  }
}));

describe('AccessControlPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(api.auditLogs).mockResolvedValue([]);
    vi.mocked(api.rotateAccessControl).mockResolvedValue({
      topic_id: 'kukuri:topic:test',
      scope: 'invite',
      previous_epoch: 1,
      new_epoch: 2,
      recipients: 5
    });
    vi.mocked(api.revokeAccessControl).mockResolvedValue({
      topic_id: 'kukuri:topic:test',
      scope: 'invite',
      revoked_pubkey: 'a'.repeat(64),
      previous_epoch: 2,
      new_epoch: 3,
      recipients: 4
    });
  });

  it('rotate と revoke の操作を送信できる', async () => {
    renderWithQueryClient(<AccessControlPage />);

    expect(await screen.findByRole('heading', { name: 'Access Control' })).toBeInTheDocument();

    const user = userEvent.setup();
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
  });
});
