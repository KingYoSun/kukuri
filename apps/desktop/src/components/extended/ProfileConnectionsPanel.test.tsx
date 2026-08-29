import { fireEvent, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, test, vi } from 'vitest';

import { ProfileConnectionsPanel } from './ProfileConnectionsPanel';

const AUTHOR_ID = 'a'.repeat(64);

function renderPanel() {
  render(
    <ProfileConnectionsPanel
      activeView='following'
      items={[
        {
          author_pubkey: AUTHOR_ID,
          name: 'alice',
          display_name: 'Alice',
          about: 'Maintains the desktop client.',
          picture: null,
          picture_asset: null,
          following: true,
          followed_by: false,
          mutual: false,
          friend_of_friend: false,
          friend_of_friend_via_pubkeys: [],
          muted: false,
          blocking: false,
          blocked_by: false,
          provenance: null,
        },
      ]}
      localAuthorPubkey={'f'.repeat(64)}
      status='ready'
      error={null}
      onSelectView={vi.fn()}
      onToggleRelationship={vi.fn()}
      onToggleMute={vi.fn()}
      onBack={vi.fn()}
    />
  );
}

test('connection rows hide author IDs and copy the complete value from context actions', async () => {
  const user = userEvent.setup();
  const clipboardWriteText = vi.fn().mockResolvedValue(undefined);
  Object.defineProperty(navigator, 'clipboard', {
    configurable: true,
    value: { writeText: clipboardWriteText },
  });
  renderPanel();

  expect(screen.queryByText(AUTHOR_ID)).not.toBeInTheDocument();
  const target = screen.getByTestId('profile-connection-identifier-target');
  fireEvent.contextMenu(target, { clientX: 32, clientY: 48 });
  await user.click(screen.getByRole('menuitem', { name: 'Copy author ID' }));
  expect(clipboardWriteText).toHaveBeenLastCalledWith(AUTHOR_ID);

  target.focus();
  fireEvent.keyDown(target, { key: 'F10', shiftKey: true });
  await user.click(screen.getByRole('menuitem', { name: 'Copy author ID' }));
  expect(clipboardWriteText).toHaveBeenCalledTimes(2);
});
