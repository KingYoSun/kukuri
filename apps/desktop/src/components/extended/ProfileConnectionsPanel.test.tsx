import { fireEvent, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, test, vi } from 'vitest';

import { ProfileConnectionsPanel } from './ProfileConnectionsPanel';

const AUTHOR_ID = 'a'.repeat(64);

test('keeps only the selected relationship action visible and opens other actions from the row menu', async () => {
  const user = userEvent.setup();
  const onToggleMute = vi.fn();
  renderPanel({ onToggleMute });
  expect(screen.queryByRole('heading')).not.toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Unfollow' })).toBeVisible();
  expect(screen.queryByRole('button', { name: 'Mute' })).not.toBeInTheDocument();
  const menu = screen.getByRole('button', { name: 'Actions for Alice' });
  await user.click(menu);
  await user.click(screen.getByRole('menuitem', { name: 'Mute' }));
  expect(onToggleMute).toHaveBeenCalledWith(AUTHOR_ID, false);
  expect(menu).toHaveFocus();
  await user.click(menu);
  await user.keyboard('{Escape}');
  expect(screen.queryByRole('menu')).not.toBeInTheDocument();
  expect(menu).toHaveFocus();
});

type PanelProps = Partial<React.ComponentProps<typeof ProfileConnectionsPanel>>;

function renderPanel(overrides: PanelProps = {}) {
  render(
    <ProfileConnectionsPanel
      activeView='following'
      items={[
        {
          author_pubkey: AUTHOR_ID,
          name: 'alice',
          display_name: 'Alice',
          about: 'Maintains the desktop client.',
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
      onToggleBlock={vi.fn()}
      onBack={vi.fn()}
      {...overrides}
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
  await user.click(screen.getByRole('menuitem', { name: 'Copy user ID' }));
  expect(clipboardWriteText).toHaveBeenLastCalledWith(AUTHOR_ID);

  target.focus();
  fireEvent.keyDown(target, { key: 'F10', shiftKey: true });
  await user.click(screen.getByRole('menuitem', { name: 'Copy user ID' }));
  expect(clipboardWriteText).toHaveBeenCalledTimes(2);
});

// #961: ブロック一覧タブと、各行からのブロック／解除操作。
test('connection menus expose block and the blocked tab remains available', async () => {
  const user = userEvent.setup();
  const onToggleBlock = vi.fn();
  const onSelectView = vi.fn();
  renderPanel({ onToggleBlock, onSelectView });

  expect(screen.getByRole('tab', { name: 'Blocked' })).toHaveAttribute('aria-selected', 'false');
  await user.click(screen.getByRole('button', { name: 'Actions for Alice' }));
  await user.click(screen.getByRole('menuitem', { name: 'Block' }));
  expect(onToggleBlock).toHaveBeenCalledWith(AUTHOR_ID, false);

  await user.click(screen.getByRole('tab', { name: 'Blocked' }));
  expect(onSelectView).toHaveBeenCalledWith('blocking');
});

test('the blocked tab shows an unblock action, a blocked badge, and its own empty state', async () => {
  const user = userEvent.setup();
  const onToggleBlock = vi.fn();
  const { unmount } = render(
    <ProfileConnectionsPanel
      activeView='blocking'
      items={[
        {
          author_pubkey: AUTHOR_ID,
          name: 'alice',
          display_name: 'Alice',
          about: null,
          picture_asset: null,
          following: false,
          followed_by: false,
          mutual: false,
          friend_of_friend: false,
          friend_of_friend_via_pubkeys: [],
          muted: false,
          blocking: true,
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
      onToggleBlock={onToggleBlock}
      onBack={vi.fn()}
    />
  );

  expect(screen.getByRole('tab', { name: 'Blocked' })).toHaveAttribute('aria-selected', 'true');
  expect(screen.getByText('Blocked', { selector: '.relationship-badge' })).toBeInTheDocument();
  await user.click(screen.getByRole('button', { name: 'Unblock' }));
  expect(onToggleBlock).toHaveBeenCalledWith(AUTHOR_ID, true);
  unmount();

  renderPanel({ activeView: 'blocking', items: [] });
  expect(screen.getByText('No blocked users yet.')).toBeInTheDocument();
});

test('the local author row never offers block', () => {
  renderPanel({ localAuthorPubkey: AUTHOR_ID });
  expect(screen.queryByRole('button', { name: 'Block' })).not.toBeInTheDocument();
  expect(screen.queryByRole('button', { name: 'Mute' })).not.toBeInTheDocument();
  expect(screen.queryByRole('button', { name: 'Actions for Alice' })).not.toBeInTheDocument();
});

test.each([
  ['following', 'Unfollow', 'onToggleRelationship', true],
  ['followed', 'Unfollow', 'onToggleRelationship', true],
  ['muted', 'Mute', 'onToggleMute', false],
  ['blocking', 'Block', 'onToggleBlock', false],
] as const)('the %s view keeps its own action beside the user information', async (activeView, label, callback, value) => {
  const user = userEvent.setup();
  const action = vi.fn();
  renderPanel({ activeView, [callback]: action });
  await user.click(screen.getByRole('button', { name: label }));
  expect(action).toHaveBeenCalledExactlyOnceWith(AUTHOR_ID, value);
  await user.click(screen.getByRole('button', { name: 'Actions for Alice' }));
  expect(screen.getAllByRole('menuitem')).toHaveLength(2);
  expect(screen.queryByRole('menuitem', { name: label })).not.toBeInTheDocument();
});
