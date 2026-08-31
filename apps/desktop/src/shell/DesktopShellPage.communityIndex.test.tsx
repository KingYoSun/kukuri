import { screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, expect, test, vi } from 'vitest';

import type { CommunityNodeManifest } from '@/lib/api';
import { createDesktopMockApi } from '@/mocks/desktopApiMock';
import { COMMUNITY_INDEX_NODE_PREFERENCE_STORAGE_KEY } from '@/shell/communityIndexNodePreference';
import { renderAtHash, setViewportWidth } from './DesktopShellPage.testHelpers';

const NODE_A = 'https://index-a.example';
const NODE_B = 'https://index-b.example';

function manifestFor(baseUrl: string, nodeName: string): CommunityNodeManifest {
  return {
    node_id: baseUrl,
    node_name: nodeName,
    node_role: 'community-node',
    server_name: new URL(baseUrl).host,
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
    report_endpoint: `${baseUrl}/v1/report`,
    terms_url: '',
    privacy_url: '',
    moderation_policy_url: '',
  };
}

beforeEach(() => {
  setViewportWidth(1024);
  window.history.replaceState(null, '', '/');
});

test('Explore header selects named eligible nodes, clears stale results, and returns to automatic', async () => {
  const user = userEvent.setup();
  const api = createDesktopMockApi();
  const indexedObjectIds = new Map<string, string>();
  for (const baseUrl of [NODE_A, NODE_B]) {
    indexedObjectIds.set(
      baseUrl,
      await api.createPost('general', `canonical post from ${baseUrl}`, null, [])
    );
  }
  await api.setCommunityNodeConfig([
    { base_url: NODE_A, auto_approve: false },
    { base_url: NODE_B, auto_approve: false },
  ]);
  for (const baseUrl of [NODE_A, NODE_B]) {
    await api.authenticateCommunityNode(baseUrl);
    await api.acceptCommunityNodeConsents(baseUrl, []);
  }
  vi.spyOn(api, 'fetchCommunityNodeManifest').mockImplementation(async (baseUrl) => ({
    status: 'ok',
    manifest: manifestFor(baseUrl, baseUrl === NODE_A ? 'Alpha Index' : 'Beta Index'),
  }));
  vi.spyOn(api, 'searchCommunityNodeIndex').mockImplementation(async (request) => ({
    entries: [
      {
        scope_kind: 'public_topic',
        scope_id: 'general',
        object_id: indexedObjectIds.get(request.base_url) ?? 'missing-index-result',
        author_pubkey: 'f'.repeat(64),
        text: `result from ${request.base_url}`,
        created_at: 1,
      },
    ],
  }));

  renderAtHash('#/explore?topic=kukuri%3Atopic%3Ageneral', api);
  const explore = await screen.findByRole('region', { name: /^Explore Column,/ });
  const nodeSelect = await within(explore).findByRole('combobox', {
    name: 'Explore Community Node',
  });
  await waitFor(() => {
    expect(within(nodeSelect).getByRole('option', { name: 'Alpha Index' })).toBeInTheDocument();
    expect(within(nodeSelect).getByRole('option', { name: 'Beta Index' })).toBeInTheDocument();
  });
  expect(nodeSelect).toHaveValue('automatic');

  await user.selectOptions(nodeSelect, NODE_B);
  await waitFor(() => {
    expect(window.localStorage.getItem(COMMUNITY_INDEX_NODE_PREFERENCE_STORAGE_KEY)).toContain(
      `"baseUrl":"${NODE_B}"`
    );
  });
  await user.type(within(explore).getByLabelText('Search query'), 'hello');
  await user.click(within(explore).getByRole('button', { name: 'Run' }));
  const result = await within(explore).findByText(`result from ${NODE_B}`);
  const resultCard = result.closest('article');
  if (!(resultCard instanceof HTMLElement)) throw new Error('Explore result card not found');
  expect(await within(resultCard).findByRole('button', { name: 'React' })).toBeEnabled();
  expect(within(resultCard).getByRole('button', { name: 'Repost' })).toBeInTheDocument();
  expect(within(resultCard).getByRole('button', { name: 'Reply' })).toBeInTheDocument();
  expect(within(resultCard).getByRole('button', { name: 'Copy link' })).toBeInTheDocument();
  expect(within(resultCard).getByRole('button', { name: 'Bookmark' })).toBeInTheDocument();
  expect(within(resultCard).getByRole('button', { name: 'Report' })).toBeInTheDocument();

  await user.selectOptions(nodeSelect, NODE_A);
  await waitFor(() => {
    expect(within(explore).queryByText(`result from ${NODE_B}`)).not.toBeInTheDocument();
    expect(within(explore).queryByRole('button', { name: 'Report' })).not.toBeInTheDocument();
  });

  await user.selectOptions(nodeSelect, 'automatic');
  await waitFor(() => {
    expect(window.localStorage.getItem(COMMUNITY_INDEX_NODE_PREFERENCE_STORAGE_KEY)).toContain(
      '"mode":"auto"'
    );
  });
});
