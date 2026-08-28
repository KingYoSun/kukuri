import { act, fireEvent, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';

import { createDesktopMockApi } from '@/mocks/desktopApiMock';
import type { PostView } from '@/lib/api';
import type { ColumnState } from '@/shell/slices/workspace';
import { WORKSPACE_LAYOUT_STORAGE_KEY } from '@/shell/workspacePersistence';
import { renderAtHash, setViewportWidth } from './DesktopShellPage.testHelpers';

// 監査 B2: Column header の非 interactive 領域 pointerdown による再アクティブ化で
// 同期 effect が再実行されると、child Column の parentColumnId が自分自身を指してしまい、
// 以後「同じ parent の一時 Column は置き換える」ルールから外れて Column が蓄積する問題の回帰テスト。

const ALICE = 'a'.repeat(64);
const BOB = 'b'.repeat(64);

function buildAuthorPost(
  objectId: string,
  authorPubkey: string,
  authorName: string,
  content: string,
  createdAt: number
): PostView {
  return {
    object_id: objectId,
    envelope_id: `envelope-${objectId}`,
    author_pubkey: authorPubkey,
    author_name: authorName,
    author_display_name: null,
    following: false,
    followed_by: false,
    mutual: false,
    friend_of_friend: false,
    object_kind: 'post',
    is_threadable: true,
    content,
    content_status: 'Available',
    attachments: [],
    created_at: createdAt,
    reply_to: null,
    root_id: objectId,
    channel_id: null,
    audience_label: 'Public',
  };
}

function readPersistedColumns(): ColumnState[] {
  const raw = window.localStorage.getItem(WORKSPACE_LAYOUT_STORAGE_KEY);
  if (!raw) throw new Error('workspace layout is not persisted yet');
  return (JSON.parse(raw) as { columns: ColumnState[] }).columns;
}

async function settle(ms = 300) {
  await act(async () => {
    await new Promise((resolve) => setTimeout(resolve, ms));
  });
}

function getColumn(title: 'Timeline' | 'Profile') {
  const columns = screen.getAllByRole('region', { name: new RegExp(`^${title} Column,`) });
  return columns.find((column) => column.getAttribute('aria-current') === 'true') ?? columns[0];
}

function pointerDownOnHeader(column: HTMLElement, title: 'Timeline' | 'Profile') {
  fireEvent.pointerDown(within(column).getByRole('heading', { level: 2, name: title }));
}

beforeEach(() => {
  setViewportWidth(1024);
  window.history.replaceState(null, '', '/');
});

afterEach(() => {
  vi.useRealTimers();
});

test('Profile header 再アクティブ化後に別 author を開くと Profile Column が置き換わる', async () => {
  const user = userEvent.setup();
  const api = createDesktopMockApi({
    seedPosts: {
      'kukuri:topic:general': [
        buildAuthorPost('post-alice', ALICE, 'alice', 'alice says hello', 2),
        buildAuthorPost('post-bob', BOB, 'bob', 'bob says hello', 1),
      ],
    },
    authorSocialViews: {
      [ALICE]: { name: 'alice' },
      [BOB]: { name: 'bob' },
    },
  });

  renderAtHash('#/timeline?topic=kukuri%3Atopic%3Ageneral', api);

  // 1. Timeline の alice を開く → Profile Column(parent = timeline)
  const timeline = await screen.findByRole('region', { name: /^Timeline Column,/ });
  await user.click(within(timeline).getByRole('button', { name: 'alice' }));
  await waitFor(() => {
    expect(
      readPersistedColumns().filter(
        (column) => column.kind === 'profile' && column.entityId === ALICE
      )
    ).toHaveLength(1);
  });
  await settle();

  const timelineColumnId = timeline.dataset.columnId;
  expect(timelineColumnId).toBeTruthy();
  const aliceColumn = readPersistedColumns().find(
    (column) => column.kind === 'profile' && column.entityId === ALICE
  );
  expect(aliceColumn?.parentColumnId).toBe(timelineColumnId);

  // 2. Timeline header(非 interactive)を pointerdown → Timeline が active
  pointerDownOnHeader(getColumn('Timeline'), 'Timeline');
  await waitFor(() => {
    expect(getColumn('Timeline')).toHaveAttribute('aria-current', 'true');
  });
  await settle();

  // 3. Profile header(非 interactive)を pointerdown → Profile が active に戻るが parent は timeline のまま
  pointerDownOnHeader(getColumn('Profile'), 'Profile');
  await waitFor(() => {
    expect(getColumn('Profile')).toHaveAttribute('aria-current', 'true');
  });
  await settle();

  const aliceAfterReactivate = readPersistedColumns().find(
    (column) => column.kind === 'profile' && column.entityId === ALICE
  );
  expect(aliceAfterReactivate?.parentColumnId).toBe(timelineColumnId);
  expect(aliceAfterReactivate?.parentColumnId).not.toBe(aliceAfterReactivate?.id);

  // 4. Timeline から bob を開く → alice が bob に置き換わり、自分用Profileは維持される
  await user.click(within(getColumn('Timeline')).getByRole('button', { name: 'bob' }));
  await waitFor(() => {
    expect(
      readPersistedColumns().some(
        (column) => column.kind === 'profile' && column.entityId === BOB
      )
    ).toBe(true);
  });
  await settle();

  const profileColumns = readPersistedColumns().filter((column) => column.kind === 'profile');
  expect(profileColumns.map((column) => column.entityId)).toEqual([BOB, undefined]);
  expect(profileColumns[0]?.parentColumnId).toBe(timelineColumnId);
  expect(profileColumns[1]?.parentColumnId).toBeUndefined();
  expect(screen.getAllByRole('region', { name: /^Profile Column,/ })).toHaveLength(2);
});
