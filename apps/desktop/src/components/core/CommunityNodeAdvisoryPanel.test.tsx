import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, test, vi } from 'vitest';

import { InvokeError } from '@/lib/api/invoke/error';

import { CommunityNodeAdvisoryPanel } from './CommunityNodeAdvisoryPanel';

const targetPubkey = 'a'.repeat(64);
const otherTargetPubkey = 'd'.repeat(64);
const nodeA = 'https://node.example';
const nodeB = 'https://node-b.example';

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}

function trustResponse(target: string, issuerNodeId: string) {
  return {
    viewer_pubkey: 'viewer',
    target_id: target,
    absolute: 0.4,
    relative: 0.6,
    trust: 0.5,
    w_abs_applied: 0.5,
    computed_at: '2026-08-14T00:00:00Z',
    basis: [
      {
        signal_id: `signal-${issuerNodeId}`,
        issuer_node_id: issuerNodeId,
        target: 'user_pubkey' as const,
        target_id: target,
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
  };
}

function relationResponse(target: string, score = 0.42) {
  return {
    viewer_pubkey: 'viewer',
    target_pubkey: target,
    score,
    basis: [{ feature: 'shared_topics', value: 1, weight: score, contribution: score }],
  };
}

function neighborsResponse(pubkey: string) {
  return { viewer_pubkey: 'viewer', neighbors: [pubkey] };
}

function api(
  signalTarget: 'user_pubkey' | 'post_id' = 'user_pubkey',
  signalTargetId = targetPubkey,
  initialAppealStatus: 'none' | 'disputed' | 'cleared' = 'none'
) {
  let appealStatus: 'none' | 'disputed' | 'cleared' = initialAppealStatus;
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
          target: signalTarget,
          target_id: signalTargetId,
          component: 'relative' as const,
          category: 'spam' as const,
          severity: 'low' as const,
          basis: 'provider_verdict' as const,
          confidence: 0.75,
          visibility: 'subscribed_nodes' as const,
          appeal_status: appealStatus,
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
    submitCommunityNodeReport: vi.fn(async (request: { appeal?: { risk_signal_id: string } | null }) => {
      appealStatus = 'disputed';
      return {
        status: 'submitted' as const,
        reference_id: 'report-1',
        disputed_risk_signal_id: request.appeal?.risk_signal_id ?? null,
      };
    }),
  };
}

const communityNodeManifests = {
  [nodeA]: {
    node_id: 'node-a',
    node_name: 'ノード A',
    node_role: 'community-node',
    server_name: 'node.example',
    manifest_version: 'v1',
    capability_scope: { available_enabled: ['trust_signal'], planned_enabled: [] },
    authority_scope: { applies_to: ['this_node'], does_not_apply_to: [] },
    p2p_boundary: {
      identity_authority: false,
      profile_canonical_store: false,
      social_graph_canonical_store: false,
      content_truth_source: false,
      network_wide_authority: false,
    },
    abuse_contact: '',
    report_endpoint: 'https://node.example/v1/report',
    terms_url: '',
    privacy_url: '',
    moderation_policy_url: '',
  },
  [nodeB]: {
    node_id: 'node-b',
    node_name: 'ノード B',
    node_role: 'community-node',
    server_name: 'node-b.example',
    manifest_version: 'v1',
    capability_scope: { available_enabled: ['trust_signal'], planned_enabled: [] },
    authority_scope: { applies_to: ['this_node'], does_not_apply_to: [] },
    p2p_boundary: {
      identity_authority: false,
      profile_canonical_store: false,
      social_graph_canonical_store: false,
      content_truth_source: false,
      network_wide_authority: false,
    },
    abuse_contact: '',
    report_endpoint: `${nodeB}/v1/report`,
    terms_url: '',
    privacy_url: '',
    moderation_policy_url: '',
  },
};

describe('CommunityNodeAdvisoryPanel', () => {
  test('loads continuous trust basis, relation, and neighbors only after user action', async () => {
    const client = api();
    render(
      <CommunityNodeAdvisoryPanel
        api={client}
        targetPubkey={targetPubkey}
        nodeBaseUrls={['https://node.example']}
        communityNodeManifests={communityNodeManifests}
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
        communityNodeManifests={communityNodeManifests}
      />
    );

    await userEvent.click(screen.getByRole('button', { name: /Load advisory|情報を読み込む/ }));

    expect(await screen.findByText(/reason is intentionally not disclosed|理由は推測・開示しません/)).toBeInTheDocument();
    expect(screen.queryByText(/hidden by target opt-out/)).not.toBeInTheDocument();
  });

  test('sends an anonymous appeal only to the matching issuer and refreshes the advisory', async () => {
    const client = api();
    render(
      <CommunityNodeAdvisoryPanel
        api={client}
        targetPubkey={targetPubkey}
        nodeBaseUrls={['https://node.example']}
        communityNodeManifests={communityNodeManifests}
      />
    );

    await userEvent.click(screen.getByRole('button', { name: /Load advisory|情報を読み込む/ }));
    await userEvent.click(await screen.findByText(/node-a/));
    await userEvent.click(
      screen.getByRole('button', { name: 'このリスク判定に異議を申し立てる' })
    );

    expect(screen.queryByLabelText(/contact|連絡先/i)).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: '異議を申し立てる' }));

    expect(client.submitCommunityNodeReport).toHaveBeenCalledWith(
      expect.objectContaining({
        node_base_url: 'https://node.example',
        report_endpoint: 'https://node.example/v1/report',
        subject_kind: 'profile',
        subject_id: targetPubkey,
        capability: 'trust_signal',
        reporter_contact: null,
        appeal: { risk_signal_id: 'signal-1' },
      })
    );
    expect(await screen.findByText(/受付対象のリスク判定: signal-1/)).toBeInTheDocument();
    expect(client.readCommunityNodeTrustUser).toHaveBeenCalledTimes(2);
    expect(screen.queryByRole('button', { name: '異議を申し立てる' })).not.toBeInTheDocument();
  });

  test('uses the original post target when appealing a post risk judgment', async () => {
    const client = api('post_id', 'post-appealed');
    render(
      <CommunityNodeAdvisoryPanel
        api={client}
        targetPubkey={targetPubkey}
        nodeBaseUrls={['https://node.example']}
        communityNodeManifests={communityNodeManifests}
      />
    );

    await userEvent.click(screen.getByRole('button', { name: /Load advisory|情報を読み込む/ }));
    await userEvent.click(await screen.findByText(/node-a/));
    await userEvent.click(
      screen.getByRole('button', { name: 'このリスク判定に異議を申し立てる' })
    );
    await userEvent.click(screen.getByRole('button', { name: '異議を申し立てる' }));

    expect(client.submitCommunityNodeReport).toHaveBeenCalledWith(
      expect.objectContaining({
        subject_kind: 'post',
        subject_id: 'post-appealed',
        appeal: { risk_signal_id: 'signal-1' },
      })
    );
  });

  test('shows a cleared post judgment as resolved without another appeal action', async () => {
    const client = api('post_id', 'post-cleared', 'cleared');
    render(
      <CommunityNodeAdvisoryPanel
        api={client}
        targetPubkey={targetPubkey}
        nodeBaseUrls={['https://node.example']}
        communityNodeManifests={communityNodeManifests}
      />
    );

    await userEvent.click(screen.getByRole('button', { name: /Load advisory|情報を読み込む/ }));
    await userEvent.click(await screen.findByText(/node-a/));

    expect(screen.getByText('認容済み')).toBeInTheDocument();
    expect(screen.getByText(/評価への寄与から除外されています/)).toBeInTheDocument();
    expect(screen.getByText(/投稿 · post-cleared/)).toBeInTheDocument();
    expect(
      screen.queryByRole('button', { name: 'このリスク判定に異議を申し立てる' })
    ).not.toBeInTheDocument();
  });

  test('clears loaded results and an open appeal when the target author changes', async () => {
    const client = api();
    const { rerender } = render(
      <CommunityNodeAdvisoryPanel
        api={client}
        targetPubkey={targetPubkey}
        nodeBaseUrls={[nodeA]}
        communityNodeManifests={communityNodeManifests}
      />
    );

    await userEvent.click(screen.getByRole('button', { name: /Load advisory|情報を読み込む/ }));
    await userEvent.click(await screen.findByText(/node-a/));
    await userEvent.click(
      screen.getByRole('button', { name: 'このリスク判定に異議を申し立てる' })
    );
    expect(screen.getByRole('dialog')).toBeInTheDocument();

    rerender(
      <CommunityNodeAdvisoryPanel
        api={client}
        targetPubkey={otherTargetPubkey}
        nodeBaseUrls={[nodeA]}
        communityNodeManifests={communityNodeManifests}
      />
    );

    await waitFor(() => expect(screen.queryAllByText(/node-a/)).toHaveLength(0));
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  test('clears an open appeal when the selected node changes', async () => {
    const client = api();
    render(
      <CommunityNodeAdvisoryPanel
        api={client}
        targetPubkey={targetPubkey}
        nodeBaseUrls={[nodeA, nodeB]}
        communityNodeManifests={communityNodeManifests}
      />
    );

    await userEvent.click(screen.getByRole('button', { name: /Load advisory|情報を読み込む/ }));
    await userEvent.click(await screen.findByText(/node-a/));
    await userEvent.click(
      screen.getByRole('button', { name: 'このリスク判定に異議を申し立てる' })
    );
    fireEvent.change(screen.getByRole('combobox', { hidden: true }), { target: { value: nodeB } });

    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
    expect(screen.queryAllByText(/node-a/)).toHaveLength(0);
  });

  test('discards a pending response after the target author changes', async () => {
    const pendingTrust = deferred<ReturnType<typeof trustResponse>>();
    const pendingRelation = deferred<ReturnType<typeof relationResponse>>();
    const pendingNeighbors = deferred<ReturnType<typeof neighborsResponse>>();
    const client = {
      readCommunityNodeTrustUser: vi.fn(() => pendingTrust.promise),
      readCommunityNodeRelationUser: vi.fn(() => pendingRelation.promise),
      listCommunityNodeRelationNeighbors: vi.fn(() => pendingNeighbors.promise),
      submitCommunityNodeReport: vi.fn(),
    };
    const { rerender } = render(
      <CommunityNodeAdvisoryPanel
        api={client}
        targetPubkey={targetPubkey}
        nodeBaseUrls={[nodeA]}
        communityNodeManifests={communityNodeManifests}
      />
    );

    await userEvent.click(screen.getByRole('button', { name: /Load advisory|情報を読み込む/ }));
    rerender(
      <CommunityNodeAdvisoryPanel
        api={client}
        targetPubkey={otherTargetPubkey}
        nodeBaseUrls={[nodeA]}
        communityNodeManifests={communityNodeManifests}
      />
    );
    pendingTrust.resolve(trustResponse(targetPubkey, 'stale-node-a'));
    pendingRelation.resolve(relationResponse(targetPubkey));
    pendingNeighbors.resolve(neighborsResponse('b'.repeat(64)));

    await waitFor(() => expect(client.readCommunityNodeTrustUser).toHaveBeenCalledTimes(1));
    expect(screen.queryByText(/stale-node-a/)).not.toBeInTheDocument();
    expect(screen.queryByText('b'.repeat(64))).not.toBeInTheDocument();
  });

  test('keeps the current author result when responses complete in reverse order', async () => {
    const trustA = deferred<ReturnType<typeof trustResponse>>();
    const trustB = deferred<ReturnType<typeof trustResponse>>();
    const relationA = deferred<ReturnType<typeof relationResponse>>();
    const relationB = deferred<ReturnType<typeof relationResponse>>();
    const neighborsA = deferred<ReturnType<typeof neighborsResponse>>();
    const neighborsB = deferred<ReturnType<typeof neighborsResponse>>();
    const client = {
      readCommunityNodeTrustUser: vi.fn((request: { target_pubkey: string }) =>
        request.target_pubkey === targetPubkey ? trustA.promise : trustB.promise
      ),
      readCommunityNodeRelationUser: vi.fn((request: { target_pubkey: string }) =>
        request.target_pubkey === targetPubkey ? relationA.promise : relationB.promise
      ),
      listCommunityNodeRelationNeighbors: vi
        .fn()
        .mockImplementationOnce(() => neighborsA.promise)
        .mockImplementationOnce(() => neighborsB.promise),
      submitCommunityNodeReport: vi.fn(),
    };
    const { rerender } = render(
      <CommunityNodeAdvisoryPanel
        api={client}
        targetPubkey={targetPubkey}
        nodeBaseUrls={[nodeA]}
        communityNodeManifests={communityNodeManifests}
      />
    );

    await userEvent.click(screen.getByRole('button', { name: /Load advisory|情報を読み込む/ }));
    rerender(
      <CommunityNodeAdvisoryPanel
        api={client}
        targetPubkey={otherTargetPubkey}
        nodeBaseUrls={[nodeA]}
        communityNodeManifests={communityNodeManifests}
      />
    );
    await userEvent.click(screen.getByRole('button', { name: /Load advisory|情報を読み込む/ }));

    trustB.resolve(trustResponse(otherTargetPubkey, 'current-node-b'));
    relationB.resolve(relationResponse(otherTargetPubkey, 0.73));
    neighborsB.resolve(neighborsResponse('e'.repeat(64)));
    expect(await screen.findByText(/current-node-b/)).toBeInTheDocument();

    trustA.resolve(trustResponse(targetPubkey, 'stale-node-a'));
    relationA.resolve(relationResponse(targetPubkey, 0.11));
    neighborsA.resolve(neighborsResponse('b'.repeat(64)));

    await waitFor(() => expect(client.readCommunityNodeTrustUser).toHaveBeenCalledTimes(2));
    expect(screen.queryByText(/stale-node-a/)).not.toBeInTheDocument();
    expect(screen.getByText(/current-node-b/)).toBeInTheDocument();
    expect(screen.getByText('e'.repeat(64))).toBeInTheDocument();
  });
});
