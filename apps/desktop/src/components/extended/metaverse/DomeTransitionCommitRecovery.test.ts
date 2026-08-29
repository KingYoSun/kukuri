import { describe, expect, test, vi } from 'vitest';

import type {
  DomeHostingView,
  DomeTransitionAdmissionTicketV1,
} from '@/lib/api';
import { recoverDomeTransitionCommit } from './DomeTransitionCommitRecovery';

const ticket: DomeTransitionAdmissionTicketV1 = {
  request: {
    transition_id: 'transition-1',
    connection_id: 'connection-1',
    topology_digest: 'topology-1',
    spatial_context: { kind: 'topic', topic_id: 'kukuri:topic:test' },
    source_instance_id: 'dome-a',
    source_instance_generation: 1,
    target_instance_id: 'dome-b',
    target_instance_generation: 1,
    participant_pubkey: 'f'.repeat(64),
    direction: 'north',
    requested_at: 1_000,
  },
  target_lease_epoch: 7,
  target_session_id: 'session-b',
  expires_at: 16_000,
};

function hosting(leaseEpoch = 7, sessionId = 'session-b'): DomeHostingView {
  return {
    instance_id: 'dome-b',
    state: {
      kind: 'community_node_hosted',
      host: { kind: 'community_node', node_id: 'cn-1', api_base_url: 'https://cn.example' },
      lease_id: 'lease-b',
      lease_epoch: leaseEpoch,
      lease_expires_at: 60_000,
      session_id: sessionId,
      reason: null,
      last_heartbeat_at: 1_000,
    },
    lease: null,
    signed_lease_json: null,
    signed_activation_json: null,
    signed_close_json: null,
    instance_manifest_json: '{}',
    preset_manifest_json: '{}',
    participants: 1,
    sleeping: false,
    resource_budget: {} as DomeHostingView['resource_budget'],
    resource_metrics: {} as DomeHostingView['resource_metrics'],
  };
}

describe('recoverDomeTransitionCommit', () => {
  test('keeps the same commit pending across transient errors until an acknowledgement arrives', async () => {
    const commit = vi.fn()
      .mockRejectedValueOnce(new Error('connection reset'))
      .mockRejectedValueOnce(new Error('timeout'))
      .mockResolvedValue(undefined);
    const wait = vi.fn().mockResolvedValue(undefined);

    await expect(recoverDomeTransitionCommit({
      ticket,
      commit,
      getHosting: vi.fn().mockResolvedValue(hosting()),
      isCurrent: () => true,
      wait,
    })).resolves.toEqual({ status: 'committed' });

    expect(commit).toHaveBeenCalledTimes(3);
    expect(wait).toHaveBeenNthCalledWith(1, 0);
    expect(wait).toHaveBeenNthCalledWith(2, 250);
  });

  test('rolls back only after the destination session is known to have changed', async () => {
    const error = new Error('connection reset');
    const commit = vi.fn().mockRejectedValue(error);

    await expect(recoverDomeTransitionCommit({
      ticket,
      commit,
      getHosting: vi.fn().mockResolvedValue(hosting(8, 'session-c')),
      isCurrent: () => true,
      wait: vi.fn().mockResolvedValue(undefined),
    })).resolves.toEqual({ status: 'rollback', error });

    expect(commit).toHaveBeenCalledTimes(1);
  });

  test('does not infer rollback when the hosting lookup also fails', async () => {
    const commit = vi.fn()
      .mockRejectedValueOnce(new Error('connection reset'))
      .mockResolvedValue(undefined);

    await expect(recoverDomeTransitionCommit({
      ticket,
      commit,
      getHosting: vi.fn().mockRejectedValue(new Error('offline')),
      isCurrent: () => true,
      wait: vi.fn().mockResolvedValue(undefined),
    })).resolves.toEqual({ status: 'committed' });

    expect(commit).toHaveBeenCalledTimes(2);
  });

  test('treats an authoritative invalid-ticket response as safe to roll back', async () => {
    const error = new Error('DOME_TRANSITION_INVALID_TICKET');

    await expect(recoverDomeTransitionCommit({
      ticket,
      commit: vi.fn().mockRejectedValue(error),
      getHosting: vi.fn().mockResolvedValue(hosting()),
      isCurrent: () => true,
      wait: vi.fn().mockResolvedValue(undefined),
    })).resolves.toEqual({ status: 'rollback', error });
  });
});
