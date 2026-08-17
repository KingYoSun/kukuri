import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, test, vi } from 'vitest';

import { STORY_AUTHOR_DETAIL_VIEW } from '@/components/storyFixtures';

import { AuthorDetailCard } from './AuthorDetailCard';

test('author detail marks long unbroken values as wrappable content', () => {
  const longPubkey = 'b'.repeat(96);
  const longViaPubkey = 'c'.repeat(96);
  const longAbout = `Maintains ${'connectivity'.repeat(12)}`;

  render(
    <AuthorDetailCard
      view={{
        ...STORY_AUTHOR_DETAIL_VIEW,
        author: {
          ...STORY_AUTHOR_DETAIL_VIEW.author!,
          author_pubkey: longPubkey,
          about: longAbout,
        },
        summary: {
          ...STORY_AUTHOR_DETAIL_VIEW.summary!,
          viaPubkeys: [longViaPubkey],
        },
      }}
      localAuthorPubkey={'f'.repeat(64)}
      onToggleRelationship={vi.fn()}
      onToggleMute={vi.fn()}
    />
  );

  expect(screen.queryByText('Author Detail')).not.toBeInTheDocument();
  expect(screen.getByTestId('author-detail-avatar')).toHaveClass('author-avatar-sm');
  expect(screen.getByTestId('author-detail-avatar')).toHaveTextContent('B');
  expect(screen.queryByRole('button', { name: 'Clear author' })).not.toBeInTheDocument();
  expect(screen.getByText('bob')).toHaveClass('author-detail-break');
  expect(screen.getByText(longAbout)).toHaveClass('author-detail-break');
  expect(screen.getByText(longAbout).parentElement).toHaveClass('author-detail-copy-stack');
  expect(screen.getByText(longPubkey)).toHaveClass('author-detail-monotext');
  expect(screen.getByText(longViaPubkey)).toHaveClass('author-detail-break');
  expect(screen.queryByText('following: yes')).not.toBeInTheDocument();
  expect(screen.queryByText('followed by: yes')).not.toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Unfollow' })).toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Mute' })).toBeInTheDocument();
  expect(screen.getByText('mutual').closest('.author-detail-identity')).toContainElement(
    screen.getByText('bob')
  );
  expect(screen.getByText('mutual').closest('.author-detail-actions')).toBeNull();
});

test('author report does not fetch a manifest or create a candidate without provenance', async () => {
  const user = userEvent.setup();
  const onFetchReportManifest = vi.fn();

  render(
    <AuthorDetailCard
      view={{
        ...STORY_AUTHOR_DETAIL_VIEW,
        author: {
          ...STORY_AUTHOR_DETAIL_VIEW.author!,
          provenance: null,
        },
      }}
      localAuthorPubkey={'f'.repeat(64)}
      onToggleRelationship={vi.fn()}
      onToggleMute={vi.fn()}
      onSubmitReport={vi.fn()}
      onFetchReportManifest={onFetchReportManifest}
    />
  );

  await user.click(screen.getByRole('button', { name: 'Report' }));

  expect(screen.getByRole('dialog')).toBeInTheDocument();
  expect(onFetchReportManifest).not.toHaveBeenCalled();
  expect(screen.getByText('Cannot determine a report target')).toBeInTheDocument();
  expect(screen.getByText(/block, mute, or hide this locally/i)).toBeInTheDocument();
  expect(screen.queryByRole('button', { name: 'Send report' })).not.toBeInTheDocument();
});

// #696: 著者詳細の通報も、開いた時に取得成功した最新 manifest だけから候補を作る。
function authorReportManifest(nodeId: string) {
  return {
    node_id: nodeId,
    node_name: nodeId,
    node_role: 'community-node',
    server_name: nodeId,
    manifest_version: 'v1',
    capability_scope: { available_enabled: ['community_index'], planned_enabled: [] },
    authority_scope: { applies_to: ['this_node'], does_not_apply_to: [] },
    p2p_boundary: {
      identity_authority: false,
      profile_canonical_store: false,
      social_graph_canonical_store: false,
      content_truth_source: false,
      network_wide_authority: false,
    },
    abuse_contact: '',
    report_endpoint: `https://${nodeId}/v1/report`,
    terms_url: '',
    privacy_url: '',
    moderation_policy_url: '',
  };
}

const OBSERVED_AUTHOR_VIEW = {
  ...STORY_AUTHOR_DETAIL_VIEW,
  author: {
    ...STORY_AUTHOR_DETAIL_VIEW.author!,
    provenance: {
      canonical_source: 'author_docs',
      observed_via: [
        { node_base_url: 'https://node.example', capability: 'community_index', observed_at: 1 },
      ],
    },
  },
};

test('author report fetches the observed node manifest on open and submits a profile report', async () => {
  const user = userEvent.setup();
  const onFetchReportManifest = vi
    .fn()
    .mockResolvedValue({ status: 'ok', manifest: authorReportManifest('node.example') });
  const onSubmitReport = vi.fn().mockResolvedValue({ status: 'submitted', reference_id: 'r-1' });

  render(
    <AuthorDetailCard
      view={OBSERVED_AUTHOR_VIEW}
      localAuthorPubkey={'f'.repeat(64)}
      onToggleRelationship={vi.fn()}
      onToggleMute={vi.fn()}
      onSubmitReport={onSubmitReport}
      onFetchReportManifest={onFetchReportManifest}
    />
  );

  await user.click(screen.getByRole('button', { name: 'Report' }));
  await waitFor(() => expect(onFetchReportManifest).toHaveBeenCalledWith('https://node.example'));
  expect(await screen.findByText('node.example')).toBeInTheDocument();
  await user.click(screen.getByRole('button', { name: 'Send report' }));
  await waitFor(() => expect(onSubmitReport).toHaveBeenCalledTimes(1));
  expect(onSubmitReport).toHaveBeenCalledWith(
    expect.objectContaining({
      node_base_url: 'https://node.example',
      subject_kind: 'profile',
      subject_id: STORY_AUTHOR_DETAIL_VIEW.author!.author_pubkey,
    })
  );
});

test('author report does not reuse a previously fetched target after the latest fetch fails', async () => {
  const user = userEvent.setup();
  const onFetchReportManifest = vi
    .fn()
    .mockResolvedValueOnce({ status: 'ok', manifest: authorReportManifest('node.example') })
    .mockResolvedValueOnce({ status: 'absent', manifest: null });
  const onSubmitReport = vi.fn();

  render(
    <AuthorDetailCard
      view={OBSERVED_AUTHOR_VIEW}
      localAuthorPubkey={'f'.repeat(64)}
      onToggleRelationship={vi.fn()}
      onToggleMute={vi.fn()}
      onSubmitReport={onSubmitReport}
      onFetchReportManifest={onFetchReportManifest}
    />
  );

  await user.click(screen.getByRole('button', { name: 'Report' }));
  expect(await screen.findByText('node.example')).toBeInTheDocument();
  await user.click(screen.getByRole('button', { name: 'Cancel' }));
  await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());

  await user.click(screen.getByRole('button', { name: 'Report' }));
  await waitFor(() => expect(onFetchReportManifest).toHaveBeenCalledTimes(2));
  expect(await screen.findByText(/Could not refresh report targets/)).toBeInTheDocument();
  expect(screen.queryByText('node.example')).not.toBeInTheDocument();
  expect(screen.queryByRole('button', { name: 'Send report' })).not.toBeInTheDocument();
  expect(onSubmitReport).not.toHaveBeenCalled();
});
