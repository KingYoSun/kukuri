import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { expect, test, vi } from 'vitest';

import type { CommunityNodeManifest, DesktopApi } from '@/lib/api';

import { CommunityIndexWorkspace } from './CommunityIndexWorkspace';

const manifest: CommunityNodeManifest = {
  node_id: 'node-1',
  node_name: 'Index node',
  node_role: 'community-node',
  server_name: 'index.example',
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
  abuse_contact: 'abuse@index.example',
  report_endpoint: 'https://index.example/v1/report',
  terms_url: '',
  privacy_url: '',
  moderation_policy_url: '',
};

test('topic search always sends the active public scope and labels preview text', async () => {
  const searchCommunityNodeIndex = vi.fn().mockResolvedValue({
    entries: [
      {
        scope_kind: 'public_topic',
        scope_id: 'rust',
        object_id: 'post-1',
        author_pubkey: 'author-1',
        text: 'hello\nderived-tag',
        created_at: 42,
      },
    ],
  });
  const api = { searchCommunityNodeIndex } as unknown as DesktopApi;
  render(
    <CommunityIndexWorkspace
      api={api}
      mode='topic'
      locale='en'
      activeTopic='rust'
      activeTimelineScope={{ kind: 'public' }}
      eligibleNodeBaseUrls={['https://index.example']}
      selectedNodeBaseUrl='https://index.example'
      onSelectNode={vi.fn()}
      manifests={{ 'https://index.example': { status: 'ok', manifest } }}
      onOpenCommunityNodeSettings={vi.fn()}
    />
  );

  fireEvent.change(screen.getByLabelText('Search query'), { target: { value: 'hello' } });
  fireEvent.click(screen.getByRole('button', { name: 'Run' }));

  await waitFor(() => expect(searchCommunityNodeIndex).toHaveBeenCalledTimes(1));
  expect(searchCommunityNodeIndex).toHaveBeenCalledWith(
    expect.objectContaining({
      scope_kind: 'public_topic',
      scope_id: 'rust',
      query: 'hello',
    })
  );
  expect(await screen.findByText(/Search preview; may include derived tags/)).toBeInTheDocument();
});

test('all joined topic scope is disabled without sending a query', () => {
  const api = { searchCommunityNodeIndex: vi.fn() } as unknown as DesktopApi;
  render(
    <CommunityIndexWorkspace
      api={api}
      mode='topic'
      locale='en'
      activeTopic='rust'
      activeTimelineScope={{ kind: 'all_joined' }}
      eligibleNodeBaseUrls={['https://index.example']}
      selectedNodeBaseUrl='https://index.example'
      onSelectNode={vi.fn()}
      manifests={{ 'https://index.example': { status: 'ok', manifest } }}
      onOpenCommunityNodeSettings={vi.fn()}
    />
  );
  expect(screen.getByText(/Search one public topic or private channel at a time/)).toBeInTheDocument();
  expect(screen.queryByLabelText('Search query')).not.toBeInTheDocument();
});
