import { screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, expect, test, vi } from 'vitest';

import { createDesktopMockApi } from '@/mocks/desktopApiMock';
import { renderAtHash, setViewportWidth } from './DesktopShellPage.testHelpers';

const FIRST = 'https://first.example';
const SECOND = 'https://second.example';

beforeEach(() => {
  setViewportWidth(1280);
  // 同意直後の反映をpollの成功で隠さない。
  vi.spyOn(window, 'setInterval').mockImplementation(() => 0 as unknown as ReturnType<typeof window.setInterval>);
});

test('explains the configured first node, then enables search immediately after explicit consent', async () => {
  const user = userEvent.setup();
  const api = createDesktopMockApi();
  await api.setCommunityNodeConfig([{ base_url: FIRST }, { base_url: SECOND }]);
  const policies = vi.spyOn(api, 'fetchCommunityNodePolicies');
  const accept = vi.spyOn(api, 'acceptCommunityNodeConsents');
  const authenticate = vi.spyOn(api, 'authenticateCommunityNode');
  const refresh = vi.spyOn(api, 'refreshCommunityNodeMetadata');
  const search = vi.spyOn(api, 'searchCommunityNodeIndex');
  renderAtHash('#/explore?topic=kukuri%3Atopic%3Ageneral', api);
  const intro = await screen.findByRole('dialog', { name: 'What is a community node?' });
  expect(within(intro).getByText(/First suggestion:/)).toHaveTextContent(FIRST);
  expect(policies).not.toHaveBeenCalled();
  expect(accept).not.toHaveBeenCalled();
  expect(authenticate).not.toHaveBeenCalled();
  expect(refresh).not.toHaveBeenCalled();
  expect(search).not.toHaveBeenCalled();
  await user.click(within(intro).getByRole('button', { name: 'Review terms' }));
  const terms = await screen.findByRole('dialog');
  await within(terms).findByText('You must follow the community node terms of service.');
  expect(screen.getAllByRole('dialog')).toHaveLength(1);
  expect(policies).toHaveBeenCalledWith(FIRST, 'en');
  expect(accept).not.toHaveBeenCalled();
  await user.click(within(terms).getByRole('button', { name: 'Accept' }));
  await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
  expect(accept).toHaveBeenCalledTimes(1);
  expect(accept.mock.calls[0][0]).toBe(FIRST);
  const explore = await screen.findByTestId('community-index-explore');
  const query = await within(explore).findByRole('textbox', { name: 'Search query' });
  await user.type(query, 'unmatched-test-query');
  await user.click(within(explore).getByRole('button', { name: 'Show results' }));
  expect(await within(explore).findByText('No matching posts found.')).toBeVisible();
  expect(search).toHaveBeenCalledWith(expect.objectContaining({ base_url: FIRST }));
  for (const tab of ['Discover', 'Recommendations']) {
    await user.click(within(explore).getByRole('tab', { name: tab }));
    await user.click(within(explore).getByRole('button', { name: 'Show results' }));
    expect(await within(explore).findByText('No matching posts found.')).toBeVisible();
  }
  expect((await api.getCommunityNodeStatuses())[1].local_consent?.records).toEqual([]);
});

test('Later does not accept, does not repeat, and leaves the Explore terms action usable', async () => {
  const user = userEvent.setup();
  const api = createDesktopMockApi();
  await api.setCommunityNodeConfig([{ base_url: FIRST }]);
  const accept = vi.spyOn(api, 'acceptCommunityNodeConsents');
  const fetch = vi.spyOn(api, 'fetchCommunityNodePolicies');
  const view = renderAtHash('#/explore?topic=kukuri%3Atopic%3Ageneral', api);
  const intro = await screen.findByRole('dialog', { name: 'What is a community node?' });
  await user.click(within(intro).getByRole('button', { name: 'Later' }));
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  expect(accept).not.toHaveBeenCalled();
  expect(fetch).not.toHaveBeenCalled();
  const explore = await screen.findByTestId('community-index-explore');
  await user.click(within(explore).getByRole('button', { name: 'Review terms' }));
  await within(await screen.findByRole('dialog')).findByText('You must follow the community node terms of service.');
  await user.keyboard('{Escape}');
  await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
  expect(accept).not.toHaveBeenCalled();
  view.unmount();
  renderAtHash('#/explore?topic=kukuri%3Atopic%3Ageneral', api);
  expect(await screen.findByRole('dialog', { name: 'What is a community node?' })).toBeVisible();
});

test('an accepted but offline node does not repeat the first-use explanation', async () => {
  const api = createDesktopMockApi();
  const statuses = await api.getCommunityNodeStatuses();
  vi.spyOn(api, 'getCommunityNodeStatuses').mockResolvedValue(statuses.map((status) => ({
    ...status, last_error: 'offline', session_phase: 'retrying', retry_after: Math.floor(Date.now() / 1000) + 60,
  })));
  renderAtHash('#/explore?topic=kukuri%3Atopic%3Ageneral', api);
  expect(await screen.findByText('Your consent is saved. The connection failed and is waiting to retry.')).toBeVisible();
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Check status again' })).toBeDisabled();
});

test('adding a node keeps the existing node consent and does not restart onboarding', async () => {
  const api = createDesktopMockApi();
  const existing = (await api.getCommunityNodeConfig()).nodes;
  await api.setCommunityNodeConfig([...existing, { base_url: SECOND }]);
  expect((await api.getCommunityNodeStatuses())[0].local_consent?.records.length).toBeGreaterThan(0);
  renderAtHash('#/explore?topic=kukuri%3Atopic%3Ageneral', api);
  await screen.findByRole('textbox', { name: 'Search query' });
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
});
