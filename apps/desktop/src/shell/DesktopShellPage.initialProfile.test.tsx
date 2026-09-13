import { screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, test, vi } from 'vitest';
import * as identity from '@/lib/api/identity';
import { createDesktopMockApi } from '@/mocks/desktopApiMock';
import { renderAtHash, setViewportWidth } from './DesktopShellPage.testHelpers';

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
