/**
 * Issue #765 T3: global Escape cascade の guard(jsdom 統合)。
 *
 * window レベルの Escape handler が
 * - Composer(textarea)など editable 要素からの Escape では selection を閉じない
 * - Radix Dialog が Escape を消費した場合(dismissable layer が document capture で
 *   preventDefault する)は Dialog だけが閉じ、thread selection は残る
 * - それ以外では従来どおり thread を閉じる
 * ことを DesktopShellPage 全体のマウントで固定する。
 */
import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, expect, test } from 'vitest';

import { App } from '@/App';
import { createDesktopMockApi } from '@/mocks/desktopApiMock';
import type { PostView } from '@/lib/api';
import { openChannelManager, setViewportWidth } from './DesktopShellPage.testHelpers';

const seedPost: PostView = {
  object_id: 'post-escape-guard',
  envelope_id: 'envelope-escape-guard',
  author_pubkey: 'b'.repeat(64),
  author_name: 'bob',
  author_display_name: null,
  following: false,
  followed_by: true,
  mutual: false,
  friend_of_friend: false,
  object_kind: 'post',
  is_threadable: true,
  content: 'escape guard root post',
  content_status: 'Available',
  attachments: [],
  created_at: 1,
  reply_to: null,
  root_id: 'post-escape-guard',
  channel_id: null,
  audience_label: 'Public',
};

function renderShell() {
  return render(
    <App
      api={createDesktopMockApi({
        seedPosts: { 'kukuri:topic:demo': [seedPost] },
      })}
    />
  );
}

async function openSeedThread(user: ReturnType<typeof userEvent.setup>) {
  await user.click(await screen.findByText('escape guard root post'));
  await waitFor(() => {
    expect(window.location.hash).toContain('context=thread');
  });
  const threadColumn = await screen.findByRole('region', { name: /^Thread Column,/ });
  return threadColumn;
}

beforeEach(() => {
  setViewportWidth(1024);
  window.history.replaceState(null, '', '/');
});

test('Escape inside the reply composer textarea keeps the thread selection', async () => {
  const user = userEvent.setup();
  renderShell();
  const threadColumn = await openSeedThread(user);

  await user.click(within(threadColumn).getAllByRole('button', { name: 'Reply' })[0]);
  const replyInput = await within(threadColumn).findByPlaceholderText('Write a reply');
  await user.click(replyInput);
  await user.type(replyInput, 'draft');
  await user.keyboard('{Escape}');

  // thread selection は残り、Composer の入力も失われない。
  expect(window.location.hash).toContain('context=thread');
  const survivingColumn = screen.getByRole('region', { name: /^Thread Column,/ });
  expect(within(survivingColumn).getByPlaceholderText('Write a reply')).toHaveValue('draft');
});

test('Escape closes only the Radix dialog and keeps the thread selection', async () => {
  const user = userEvent.setup();
  renderShell();
  await openSeedThread(user);

  const dialog = await openChannelManager(user);
  expect(dialog).toBeInTheDocument();
  await user.keyboard('{Escape}');

  // Radix Dialog は閉じる。
  await waitFor(() => {
    expect(
      screen.queryByRole('dialog', { name: 'Create / Join Private Channel' })
    ).not.toBeInTheDocument();
  });
  // Dialog close と同時に thread selection が解除されない。
  expect(window.location.hash).toContain('context=thread');
  expect(screen.getByRole('region', { name: /^Thread Column,/ })).toBeInTheDocument();
});

test('Escape from a non-editable target still closes the thread', async () => {
  const user = userEvent.setup();
  renderShell();
  await openSeedThread(user);

  // editable でも Dialog でもない通常状態からの Escape は従来どおり thread を閉じる。
  (document.activeElement as HTMLElement | null)?.blur();
  await user.keyboard('{Escape}');

  await waitFor(() => {
    expect(window.location.hash).not.toContain('context=thread');
  });
  // selection が解除され、focus は Timeline Column へ戻る(transient column 自体の
  // 除去は column workspace の置換規則に委ねるためここでは assert しない)。
  await waitFor(() => {
    expect(screen.getByRole('region', { name: /^Timeline Column,/ })).toHaveAttribute(
      'aria-current',
      'true'
    );
  });
});
