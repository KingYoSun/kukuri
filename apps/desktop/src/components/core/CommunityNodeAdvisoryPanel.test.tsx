import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, test, vi } from 'vitest';

import { InvokeError } from '@/lib/api/invoke/error';

import { CommunityNodeAdvisoryPanel } from './CommunityNodeAdvisoryPanel';

const targetPubkey = 'a'.repeat(64);

function api() {
  return {
    readCommunityNodeTrustUser: vi.fn(async () => ({
      viewer_pubkey: 'viewer',
      target_id: targetPubkey,
      absolute: 0.4,
      relative: 0.6,
      trust: 0.5,
      w_abs_applied: 0.5,
      computed_at: '2026-08-14T00:00:00Z',
      basis: [
        {
          signal_id: 'signal-1',
          issuer_node_id: 'node-a',
          component: 'relative' as const,
          category: 'spam' as const,
          severity: 'low' as const,
          basis: 'provider_verdict' as const,
          confidence: 0.75,
          visibility: 'subscribed_nodes' as const,
          appeal_status: 'none' as const,
          expires_at: null,
          raw_contribution: 0.5,
          decay_factor: 0.8,
          relation_weight: 1,
          contribution: 0.4,
        },
      ],
    })),
    readCommunityNodeRelationUser: vi.fn(async () => ({
      viewer_pubkey: 'viewer',
      target_pubkey: targetPubkey,
      score: 0.42,
      basis: [{ feature: 'shared_topics', value: 1, weight: 0.42, contribution: 0.42 }],
    })),
    listCommunityNodeRelationNeighbors: vi.fn(async () => ({
      viewer_pubkey: 'viewer',
      neighbors: ['b'.repeat(64)],
    })),
  };
}

describe('CommunityNodeAdvisoryPanel', () => {
  test('loads continuous trust basis, relation, and neighbors only after user action', async () => {
    const client = api();
    render(
      <CommunityNodeAdvisoryPanel
        api={client}
        targetPubkey={targetPubkey}
        nodeBaseUrls={['https://node.example']}
      />
    );

    expect(client.readCommunityNodeTrustUser).not.toHaveBeenCalled();
    await userEvent.click(screen.getByRole('button', { name: /Load advisory|情報を読み込む/ }));

    expect((await screen.findAllByText('0.500')).length).toBeGreaterThan(0);
    expect(screen.getByText(/node-a/)).toBeInTheDocument();
    expect(screen.getByText(/Continuous proximity: 0.420|連続値の proximity: 0.420/)).toBeInTheDocument();
    expect(screen.getByText('b'.repeat(64))).toBeInTheDocument();
  });

  test('shows generic relation unavailable copy without exposing an opt-out inference', async () => {
    const client = api();
    client.readCommunityNodeRelationUser.mockRejectedValue(
      new InvokeError('RELATION_NOT_FOUND', 'hidden by target opt-out')
    );
    render(
      <CommunityNodeAdvisoryPanel
        api={client}
        targetPubkey={targetPubkey}
        nodeBaseUrls={['https://node.example']}
      />
    );

    await userEvent.click(screen.getByRole('button', { name: /Load advisory|情報を読み込む/ }));

    expect(await screen.findByText(/reason is intentionally not disclosed|理由は推測・開示しません/)).toBeInTheDocument();
    expect(screen.queryByText(/hidden by target opt-out/)).not.toBeInTheDocument();
  });
});
