import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, test, vi } from 'vitest';
import { CommunityNodePanel } from './CommunityNodePanel';
import { createCommunityNodePanelFixture } from './fixtures';

test('a failed settings acceptance is explained inside the open policy dialog', async () => {
  const user = userEvent.setup();
  const view = createCommunityNodePanelFixture();
  view.nodes[0].consent = { ...view.nodes[0].consent, allRequiredAccepted: false, hasLocalConsent: false };
  const accept = vi.fn().mockRejectedValue(new Error('save failed'));
  render(<CommunityNodePanel view={view} saveDisabled resetDisabled clearDisabled={false}
    onAddNode={() => {}} onNodeBaseUrlChange={() => {}} onRemoveNode={() => {}}
    onSaveNodes={() => {}} onReset={() => {}} onClearNodes={() => {}} onAuthenticate={() => {}}
    onFetchConsents={async () => {}} onAcceptConsents={accept} onRefresh={() => {}}
    onClearToken={() => {}} onSubmitInviteCode={async () => {}}
  />);
  await user.click(screen.getAllByRole('button', { name: 'Consents' })[0]);
  const dialog = await screen.findByRole('dialog');
  await user.click(within(dialog).getByRole('button', { name: 'Accept' }));
  expect(await within(dialog).findByRole('alert')).toHaveTextContent('Consent could not be completed');
  expect(within(dialog).getByRole('button', { name: 'Retry' })).toBeEnabled();
});
