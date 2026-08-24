import { render, screen, waitFor } from '@testing-library/react';
import { beforeEach, expect, test } from 'vitest';

import { createDesktopMockApi } from '@/mocks/desktopApiMock';
import { App } from '@/App';
import { WORKSPACE_LAYOUT_STORAGE_KEY } from '@/shell/workspacePersistence';
import { columnIdentityId } from '@/shell/slices/workspace';
import { buildPaginatedPost, setViewportWidth } from './DesktopShellPage.testHelpers';

const SCOPE = { topicId: 'kukuri:topic:demo', channelId: null };
const TIMELINE_ID = columnIdentityId('timeline', SCOPE);
const THREAD_ID = columnIdentityId('thread', SCOPE, 'post-thread-open');

function seedThreadLayout(activeColumnId: string, threadEntityId = 'post-thread-open') {
  window.localStorage.setItem(
    WORKSPACE_LAYOUT_STORAGE_KEY,
    JSON.stringify({
      version: 1,
      activeColumnId,
      columns: [
        {
          id: TIMELINE_ID,
          kind: 'timeline',
          scope: SCOPE,
          pinned: true,
          preferredDesktopSpan: 1,
        },
        {
          id: columnIdentityId('thread', SCOPE, threadEntityId),
          kind: 'thread',
          scope: SCOPE,
          entityId: threadEntityId,
          parentColumnId: TIMELINE_ID,
          pinned: false,
          preferredDesktopSpan: 1,
        },
      ],
    })
  );
}

function createApi() {
  return createDesktopMockApi({
    seedPosts: {
      [SCOPE.topicId]: [
        buildPaginatedPost(1, {
          object_id: 'post-thread-open',
          envelope_id: 'envelope-thread-open',
          author_pubkey: 'b'.repeat(64),
          author_name: 'bob',
          content: 'cold start thread root',
        }),
      ],
    },
  });
}

beforeEach(() => {
  setViewportWidth(1280);
});

// Issue #765 T4: hash を持たない cold start では、復元した activeColumnId の
// canonical target を初期 route として採用し、focus を復元する。
test('cold start without a hash restores the persisted active Thread Column', async () => {
  seedThreadLayout(THREAD_ID);
  window.history.replaceState(null, '', '/');

  render(<App api={createApi()} />);

  await waitFor(() => {
    expect(
      screen.getByRole('region', { name: /^Thread Column,.*Active,/ })
    ).toBeInTheDocument();
  });
  expect(window.location.hash).toContain('context=thread');
  expect(window.location.hash).toContain('threadId=post-thread-open');
  // Timeline Column は残るが active ではない。
  expect(
    screen.getByRole('region', { name: /^Timeline Column,/ })
  ).not.toHaveAttribute('aria-current', 'true');
});

test('an explicit deep link wins over the persisted active Column', async () => {
  seedThreadLayout(THREAD_ID);
  window.history.replaceState(null, '', '/');
  window.location.hash = '#/notifications?topic=kukuri%3Atopic%3Ademo';

  render(<App api={createApi()} />);

  await waitFor(() => {
    expect(
      screen.getByRole('region', { name: /^Notifications Column,.*Active,/ })
    ).toBeInTheDocument();
  });
  expect(window.location.hash).toContain('/notifications');
  expect(window.location.hash).not.toContain('context=thread');
});

test('an invalid persisted active target falls back to the safe Timeline normalization', async () => {
  // 存在しない thread を active として保存しても、既存の安全側 normalize で Timeline に戻る。
  seedThreadLayout(columnIdentityId('thread', SCOPE, 'missing-thread'), 'missing-thread');
  window.history.replaceState(null, '', '/');

  render(<App api={createApi()} />);

  await waitFor(() => {
    expect(
      screen.getByRole('region', { name: /^Timeline Column,.*Active,/ })
    ).toBeInTheDocument();
  });
  expect(window.location.hash).not.toContain('threadId=missing-thread');
});
