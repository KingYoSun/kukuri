import { screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, test, vi } from 'vitest';
import * as identity from '@/lib/api/identity';
import { createDesktopMockApi } from '@/mocks/desktopApiMock';
import { renderAtHash, setViewportWidth } from './DesktopShellPage.testHelpers';

async function initialSetup(unconsented = false) {
  setViewportWidth(1280);
  const api = createDesktopMockApi();
  if (unconsented) await api.setCommunityNodeConfig([{ base_url: 'https://first.example' }]);
  const profile = await api.getMyProfile();
  const account = { id: profile.pubkey.slice(0, 16), pubkey: profile.pubkey, label: null, created_at: 1, last_used_at: 1 };
  vi.spyOn(identity, 'listAccounts').mockResolvedValue({ active_account_id: account.id, accounts: [account] });
  vi.spyOn(identity, 'getProfileSetupRequired').mockResolvedValue(true);
  return { api, account, user: userEvent.setup() };
}

test('initial consent failure stays in terms, interruption returns to explanation, success opens profile', async () => {
  const { api, user } = await initialSetup(true);
  const original = api.acceptCommunityNodeConsents.bind(api);
  vi.spyOn(api, 'acceptCommunityNodeConsents').mockRejectedValueOnce(new Error('offline')).mockImplementation(original);
  renderAtHash('#/explore', api);
  await user.click(within(await screen.findByRole('dialog', { name: 'What is a community node?' })).getByRole('button', { name: 'Review terms' }));
  let terms = await screen.findByRole('dialog');
  await waitFor(() => expect(within(terms).getByRole('button', { name: 'Accept' })).toBeEnabled());
  await user.click(within(terms).getByRole('button', { name: 'Accept' }));
  await within(terms).findByText(/Consent could not be completed/);
  expect(screen.queryByRole('dialog', { name: 'Set up your profile' })).not.toBeInTheDocument();
  await user.keyboard('{Escape}');
  const intro = await screen.findByRole('dialog', { name: 'What is a community node?' });
  await user.click(within(intro).getByRole('button', { name: 'Review terms' }));
  terms = await screen.findByRole('dialog');
  await waitFor(() => expect(within(terms).getByRole('button', { name: 'Accept' })).toBeEnabled());
  await user.click(within(terms).getByRole('button', { name: 'Accept' }));
  expect(await screen.findByRole('dialog', { name: 'Set up your profile' })).toBeVisible();
  expect(screen.getAllByRole('dialog')).toHaveLength(1);
});

test('initial save failure preserves editable input and retry submits the changed profile', async () => {
  const { api, account, user } = await initialSetup();
  const save = vi.spyOn(identity, 'saveInitialProfile').mockRejectedValueOnce(new Error('save failed')).mockImplementation((request) => api.setMyProfile(request.profile));
  renderAtHash('#/profile', api);
  const dialog = await screen.findByRole('dialog', { name: 'Set up your profile' });
  const name = within(dialog).getByRole('textbox', { name: 'Display Name' });
  await user.clear(name); await user.type(name, 'First attempt');
  await user.click(within(dialog).getByRole('button', { name: 'Save' }));
  await within(dialog).findByText(/Could not complete the action/);
  expect(name).toHaveValue('First attempt');
  await user.clear(name); await user.type(name, 'Edited retry');
  await user.click(within(dialog).getByRole('button', { name: 'Save' }));
  await waitFor(() => expect(dialog).not.toBeInTheDocument());
  expect(save).toHaveBeenLastCalledWith(expect.objectContaining({ account_id: account.id, profile: expect.objectContaining({ display_name: 'Edited retry' }) }));
});

test('later does not save and a new session can resume incomplete setup', async () => {
  const { api, user } = await initialSetup();
  const save = vi.spyOn(identity, 'saveInitialProfile');
  const view = renderAtHash('#/profile', api);
  await user.click(within(await screen.findByRole('dialog', { name: 'Set up your profile' })).getByRole('button', { name: 'Later' }));
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  expect(save).not.toHaveBeenCalled();
  view.unmount();
  renderAtHash('#/profile', api);
  expect(await screen.findByRole('dialog', { name: 'Set up your profile' })).toBeVisible();
});

test('offline node status needs an explicit skip before profile setup', async () => {
  const { api, user } = await initialSetup(true);
  vi.spyOn(api, 'getCommunityNodeStatuses').mockRejectedValue(new Error('offline'));
  const accept = vi.spyOn(api, 'acceptCommunityNodeConsents');
  renderAtHash('#/profile', api);
  await user.click(await screen.findByRole('button', { name: 'Skip and continue' }));
  expect(await screen.findByRole('dialog', { name: 'Set up your profile' })).toBeVisible();
  expect(accept).not.toHaveBeenCalled();
});

test('new account profile setup follows an explicit community node skip', async () => {
  setViewportWidth(1280);
  const api = createDesktopMockApi();
  await api.setCommunityNodeConfig([{ base_url: 'https://first.example' }]);
  const profile = await api.getMyProfile();
  const account = { id: profile.pubkey.slice(0, 16), pubkey: profile.pubkey, label: null, created_at: 1, last_used_at: 1 };
  vi.spyOn(identity, 'listAccounts').mockResolvedValue({ active_account_id: account.id, accounts: [account] });
  vi.spyOn(identity, 'getProfileSetupRequired').mockResolvedValue(true);
  const accept = vi.spyOn(api, 'acceptCommunityNodeConsents');
  const user = userEvent.setup();
  renderAtHash('#/explore?topic=kukuri%3Atopic%3Ageneral', api);
  const intro = await screen.findByRole('dialog', { name: 'What is a community node?' });
  expect(screen.queryByRole('dialog', { name: 'Set up your profile' })).not.toBeInTheDocument();
  await user.click(within(intro).getByRole('button', { name: 'Later' }));
  expect(await screen.findByRole('dialog', { name: 'Set up your profile' })).toBeVisible();
  expect(accept).not.toHaveBeenCalled();
});
