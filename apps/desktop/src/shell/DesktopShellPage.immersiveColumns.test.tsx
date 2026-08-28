import { act, fireEvent, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, expect, test, vi } from 'vitest';

import { createDesktopMockApi } from '@/mocks/desktopApiMock';
import type { PostView } from '@/lib/api';
import type { ColumnState } from '@/shell/slices/workspace';
import { WORKSPACE_LAYOUT_STORAGE_KEY } from '@/shell/workspacePersistence';
import { createDeferred, renderAtHash, setViewportWidth } from './DesktopShellPage.testHelpers';

// Issue #765 対象 1: Stream / Game / Metaverse の deep link は独立 transient(親なし)として開き、
// Timeline を親とする未固定 Thread Column を置換しないこと、および game rooms 未ロード時に
// metaverse 残骸 Column が発生しないことの回帰テスト。

const DEMO_TOPIC = 'kukuri:topic:general';
const TIMELINE_HASH = '#/timeline?topic=kukuri%3Atopic%3Ageneral';
const GAME_ROOM_HASH = '#/game?topic=kukuri%3Atopic%3Ageneral&roomId=room-demo';
const LIVE_SESSION_HASH = '#/live?topic=kukuri%3Atopic%3Ageneral&sessionId=session-demo';
const IMMERSIVE_KINDS: ColumnState['kind'][] = ['stream', 'game', 'metaverse'];

function buildPost(objectId: string, content: string): PostView {
  return {
    object_id: objectId,
    envelope_id: `envelope-${objectId}`,
    author_pubkey: 'b'.repeat(64),
    author_name: 'bob',
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
    created_at: 1,
    reply_to: null,
    root_id: objectId,
    channel_id: null,
    audience_label: 'Public',
  };
}

function createSeededApi() {
  return createDesktopMockApi({
    seedPosts: {
      [DEMO_TOPIC]: [buildPost('post-thread-source', 'thread source post')],
    },
    seedLiveSessions: {
      [DEMO_TOPIC]: [
        {
          session_id: 'session-demo',
          host_pubkey: 'b'.repeat(64),
          title: 'Live Demo',
          description: 'watch here',
          status: 'Live',
          started_at: 1,
          viewer_count: 1,
          joined_by_me: false,
          channel_id: null,
          audience_label: 'Public',
        },
      ],
    },
    seedGameRooms: {
      [DEMO_TOPIC]: [
        {
          room_id: 'room-demo',
          host_pubkey: 'b'.repeat(64),
          title: 'Room Demo',
          description: 'play here',
          status: 'Waiting',
          phase_label: 'Round 1',
          scores: [],
          updated_at: 1,
          channel_id: null,
          audience_label: 'Public',
        },
      ],
    },
  });
}

function readPersistedColumns(): ColumnState[] {
  const raw = window.localStorage.getItem(WORKSPACE_LAYOUT_STORAGE_KEY);
  if (!raw) throw new Error('workspace layout is not persisted yet');
  return (JSON.parse(raw) as { columns: ColumnState[] }).columns;
}

function persistedImmersiveColumns(): ColumnState[] {
  return readPersistedColumns().filter((column) => IMMERSIVE_KINDS.includes(column.kind));
}

async function settle(ms = 300) {
  await act(async () => {
    await new Promise((resolve) => setTimeout(resolve, ms));
  });
}

async function navigateHash(hash: string) {
  await act(async () => {
    window.location.hash = hash;
    // jsdom の hash 遷移が popstate を配信しない環境向けに HashRouter へ明示的に通知する。
    window.dispatchEvent(new PopStateEvent('popstate'));
  });
  await settle();
}

// Timeline の投稿本文クリックで Thread Column(parent = Timeline)を開き、
// Timeline header の pointerdown で Timeline を active に戻す。
async function openThreadThenReactivateTimeline(user: ReturnType<typeof userEvent.setup>) {
  const timeline = await screen.findByRole('region', { name: /^Timeline Column,/ });
  const postBody = within(timeline).getByText('thread source post').closest('[role="button"]');
  if (!(postBody instanceof HTMLElement)) {
    throw new Error('post body button not found');
  }
  await user.click(postBody);
  await screen.findByRole('region', { name: /^Thread Column,/ });
  await settle();

  fireEvent.pointerDown(within(timeline).getByRole('heading', { level: 2, name: 'Timeline' }));
  await waitFor(() => {
    expect(timeline).toHaveAttribute('aria-current', 'true');
  });
  await settle();
  return timeline;
}

beforeEach(() => {
  setViewportWidth(1280);
  window.history.replaceState(null, '', '/');
});

test('game room deep link keeps the unpinned Thread Column and opens an orphan Game Column', async () => {
  const user = userEvent.setup();
  renderAtHash(TIMELINE_HASH, createSeededApi());

  const timeline = await openThreadThenReactivateTimeline(user);
  const timelineColumnId = timeline.dataset.columnId;
  expect(timelineColumnId).toBeTruthy();

  await navigateHash(GAME_ROOM_HASH);

  const gameColumn = await screen.findByRole('region', { name: /^Game Column,/ });
  expect(gameColumn).toBeInTheDocument();
  // Thread Column は置換されず残る。
  expect(screen.getByRole('region', { name: /^Thread Column,/ })).toBeInTheDocument();
  await settle();

  const columns = readPersistedColumns();
  const threadColumn = columns.find((column) => column.kind === 'thread');
  expect(threadColumn?.entityId).toBe('post-thread-source');
  expect(threadColumn?.parentColumnId).toBe(timelineColumnId);
  const persistedGame = columns.find((column) => column.kind === 'game');
  expect(persistedGame?.entityId).toBe('room-demo');
  // Game Column は独立 transient(親なし)。
  expect(persistedGame?.parentColumnId).toBeUndefined();
  // metaverse 残骸 Column は発生しない。
  expect(columns.some((column) => column.kind === 'metaverse')).toBe(false);
  // 既定5 Column + Thread + Game の 7 本。
  expect(columns).toHaveLength(7);
});

test('live session deep link keeps the unpinned Thread Column and opens an orphan Live Column', async () => {
  const user = userEvent.setup();
  renderAtHash(TIMELINE_HASH, createSeededApi());

  const timeline = await openThreadThenReactivateTimeline(user);
  const timelineColumnId = timeline.dataset.columnId;
  expect(timelineColumnId).toBeTruthy();

  await navigateHash(LIVE_SESSION_HASH);

  await screen.findByRole('region', { name: /^Live Column,/ });
  expect(screen.getByRole('region', { name: /^Thread Column,/ })).toBeInTheDocument();
  await settle();

  const columns = readPersistedColumns();
  const threadColumn = columns.find((column) => column.kind === 'thread');
  expect(threadColumn?.parentColumnId).toBe(timelineColumnId);
  const streamColumn = columns.find((column) => column.kind === 'stream');
  expect(streamColumn?.entityId).toBe('session-demo');
  expect(streamColumn?.parentColumnId).toBeUndefined();
  expect(columns).toHaveLength(7);
});

test('game room deep link before rooms load ends with a single Game Column and no metaverse leftover', async () => {
  const api = createSeededApi();
  const roomsGate = createDeferred<void>();
  const originalListGameRooms = api.listGameRooms.bind(api);
  vi.spyOn(api, 'listGameRooms').mockImplementation(async (topic, scope) => {
    await roomsGate.promise;
    return originalListGameRooms(topic, scope);
  });

  renderAtHash(GAME_ROOM_HASH, api);
  await settle();

  await act(async () => {
    roomsGate.resolve();
  });
  await screen.findByRole('region', { name: /^Game Column,/ });
  await settle();

  expect(screen.queryByRole('region', { name: /^Metaverse Column,/ })).not.toBeInTheDocument();
  const immersiveColumns = persistedImmersiveColumns();
  expect(immersiveColumns.map((column) => [column.kind, column.entityId])).toEqual([
    ['game', 'room-demo'],
  ]);
});

test('switching from a live deep link to a game deep link replaces the immersive transient Column', async () => {
  renderAtHash(LIVE_SESSION_HASH, createSeededApi());

  await screen.findByRole('region', { name: /^Live Column,/ });
  await settle();

  await navigateHash(GAME_ROOM_HASH);

  await screen.findByRole('region', { name: /^Game Column,/ });
  await settle();

  expect(screen.queryByRole('region', { name: /^Live Column,/ })).not.toBeInTheDocument();
  const immersiveColumns = persistedImmersiveColumns();
  expect(immersiveColumns.map((column) => [column.kind, column.entityId])).toEqual([
    ['game', 'room-demo'],
  ]);
});
