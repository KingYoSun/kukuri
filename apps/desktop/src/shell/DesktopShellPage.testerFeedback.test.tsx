import { screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, expect, test, vi } from 'vitest';

import { createDesktopMockApi } from '@/mocks/desktopApiMock';
import { renderAtHash, setViewportWidth } from './DesktopShellPage.testHelpers';

const NODE = 'https://feedback.example';

beforeEach(() => {
  setViewportWidth(1280);
  window.history.replaceState(null, '', '/');
});

test('explains why a searchable node cannot accept feedback and opens its settings without sending', async () => {
  const user = userEvent.setup();
  const api = createDesktopMockApi();
  await api.setCommunityNodeConfig([{ base_url: NODE }]);
  await api.authenticateCommunityNode(NODE);
  await api.acceptCommunityNodeConsents(NODE, (await api.fetchCommunityNodePolicies(NODE)).policies, 'en');
  const manifest = (await api.fetchCommunityNodeManifest(NODE)).manifest!;
  vi.spyOn(api, 'fetchCommunityNodeManifest').mockResolvedValue({ status: 'ok', manifest: {
    ...manifest, node_name: 'Search Only',
    capability_scope: { available_enabled: ['community_index'], planned_enabled: [] },
  } });
  const submit = vi.spyOn(api, 'submitCommunityNodeTesterFeedback');
  const authenticate = vi.spyOn(api, 'authenticateCommunityNode');
  const accept = vi.spyOn(api, 'acceptCommunityNodeConsents');
  const save = vi.spyOn(api, 'setCommunityNodeConfig');

  renderAtHash('#/timeline', api);
  await user.click(await screen.findByTestId('tester-feedback-trigger'));
  const dialog = await screen.findByRole('dialog', { name: 'Send feedback' });
  expect(await within(dialog).findByText('Search Only')).toBeInTheDocument();
  expect(within(dialog).getByText(/This node does not offer feedback intake/)).toBeInTheDocument();
  expect(within(dialog).getByText(/Search and consent alone do not enable feedback/)).toBeInTheDocument();
  expect(within(dialog).getByRole('button', { name: 'Send' })).toBeDisabled();
  await user.click(within(dialog).getByRole('button', { name: 'Open Community Node settings' }));
  const settings = await screen.findByRole('dialog', { name: 'Settings' });
  await waitFor(() => expect(screen.getAllByRole('dialog')).toHaveLength(1));
  expect(within(settings).getByTestId('settings-section-community-node')).toHaveAttribute('aria-current', 'location');
  expect(submit).not.toHaveBeenCalled();
  expect(authenticate).not.toHaveBeenCalled();
  expect(accept).not.toHaveBeenCalled();
  expect(save).not.toHaveBeenCalled();
});
