import { screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, expect, test, vi } from 'vitest';

import { createDesktopMockApi } from '@/mocks/desktopApiMock';
import type { DesktopApi, PostView } from '@/lib/api';

import {
  buildImagePost,
  openControlCenter,
  renderAtHash,
  setViewportWidth,
} from './DesktopShellPage.testHelpers';

// #1052 / ADR 0046: 「見つける」Column で解決済み投稿の添付が、タイムラインと同じ表示経路
// (取得 -> object URL -> PostMedia)と同じ成人向けゲートで扱われることを固定する。
// 索引対象の投稿は表示中のタイムライン(general)に存在しない dev topic へ置き、
// Explore の解決済み投稿だけがメディア取得の起点になることを分離して確認する。

const IMAGE_HASH = 'a'.repeat(64);
const INDEXED_TOPIC = 'kukuri:topic:dev';
const OBJECT_ID = 'explore-image-post';

function indexedImagePost(overrides?: Partial<PostView>): PostView {
  return buildImagePost({
    object_id: OBJECT_ID,
    content: 'explore image caption',
    content_status: 'Available',
    root_id: OBJECT_ID,
    ...overrides,
  });
}

function createIndexedApi(post: PostView): DesktopApi {
  const api = createDesktopMockApi({
    seedPosts: {
      [INDEXED_TOPIC]: [post],
    },
  });
  vi.spyOn(api, 'searchCommunityNodeIndex').mockResolvedValue({
    entries: [
      {
        scope_kind: 'public_topic',
        scope_id: INDEXED_TOPIC,
        object_id: post.object_id,
        author_pubkey: post.author_pubkey,
        text: 'indexed text',
        created_at: 1,
        content_advisories: [],
      },
    ],
  });
  return api;
}

function exploreColumn() {
  return screen.getByRole('region', { name: /^Explore Column,/ });
}

async function openExploreResults(user: ReturnType<typeof userEvent.setup>, api: DesktopApi) {
  renderAtHash('#/explore?topic=kukuri%3Atopic%3Ageneral', api);
  const explore = await screen.findByRole('region', { name: /^Explore Column,/ });
  await user.type(within(explore).getByLabelText('Search query'), 'media');
  await user.click(within(explore).getByRole('button', { name: 'Show results' }));
  return explore;
}

/// 「見つける」を開いたまま安全設定を切り替える。共有ヘルパーの `openSettingsDrawer` は
/// タイムライン系の hash を前提にするため、ここでは Control Center から直接開く。
async function toggleAdultContentDisplay(user: ReturnType<typeof userEvent.setup>) {
  const controlCenter = await openControlCenter(user);
  await user.click(within(controlCenter).getByRole('button', { name: 'Settings' }));
  const drawer = await screen.findByRole('dialog', { name: 'Settings' });
  await user.click(within(drawer).getByTestId('settings-section-safety'));
  await user.click(within(drawer).getByTestId('adult-content-display-toggle'));
  await user.keyboard('{Escape}');
  await waitFor(() => {
    expect(screen.queryByRole('dialog', { name: 'Settings' })).not.toBeInTheDocument();
  });
}

beforeEach(() => {
  setViewportWidth(1280);
  window.history.replaceState(null, '', '/');
});

test('resolved Explore results fetch and render their attachments', async () => {
  const user = userEvent.setup();
  const api = createIndexedApi(indexedImagePost());
  const getBlobMediaPayload = vi.fn(api.getBlobMediaPayload);
  api.getBlobMediaPayload = getBlobMediaPayload;

  const explore = await openExploreResults(user, api);

  expect(await within(explore).findByText('explore image caption')).toBeInTheDocument();
  await waitFor(() => {
    expect(getBlobMediaPayload.mock.calls.some(([hash]) => hash === IMAGE_HASH)).toBe(true);
  });
  expect(await within(explore).findByTestId(`media-preview-${OBJECT_ID}`)).toBeInTheDocument();
});

test('adult-labeled Explore results stay gated and never request their media', async () => {
  const user = userEvent.setup();
  const api = createIndexedApi(indexedImagePost({ content_labels: ['adult'] }));
  const getBlobMediaPayload = vi.fn(api.getBlobMediaPayload);
  api.getBlobMediaPayload = getBlobMediaPayload;

  const explore = await openExploreResults(user, api);

  expect(await within(explore).findByTestId(`media-adult-gated-${OBJECT_ID}`)).toBeInTheDocument();
  expect(within(explore).queryByText('explore image caption')).not.toBeInTheDocument();
  await waitFor(() => {
    expect(getBlobMediaPayload.mock.calls.filter(([hash]) => hash === IMAGE_HASH)).toHaveLength(0);
  });
});

test('enabling adult display renders Explore media and disabling clears it again', async () => {
  const user = userEvent.setup();
  const api = createIndexedApi(indexedImagePost({ content_labels: ['adult'] }));
  const getBlobMediaPayload = vi.fn(api.getBlobMediaPayload);
  api.getBlobMediaPayload = getBlobMediaPayload;
  const setAdultContentDisplayEnabled = vi.fn(api.setAdultContentDisplayEnabled);
  api.setAdultContentDisplayEnabled = setAdultContentDisplayEnabled;

  const explore = await openExploreResults(user, api);
  expect(await within(explore).findByTestId(`media-adult-gated-${OBJECT_ID}`)).toBeInTheDocument();
  expect(getBlobMediaPayload.mock.calls.filter(([hash]) => hash === IMAGE_HASH)).toHaveLength(0);

  await toggleAdultContentDisplay(user);
  await waitFor(() => {
    expect(setAdultContentDisplayEnabled).toHaveBeenCalledWith(true);
  });
  await waitFor(() => {
    expect(getBlobMediaPayload.mock.calls.some(([hash]) => hash === IMAGE_HASH)).toBe(true);
  });
  expect(
    await within(exploreColumn()).findByTestId(`media-preview-${OBJECT_ID}`)
  ).toBeInTheDocument();

  const callsBeforeDisable = getBlobMediaPayload.mock.calls.filter(
    ([hash]) => hash === IMAGE_HASH
  ).length;
  await toggleAdultContentDisplay(user);
  await waitFor(() => {
    expect(setAdultContentDisplayEnabled).toHaveBeenCalledWith(false);
  });
  expect(
    await within(exploreColumn()).findByTestId(`media-adult-gated-${OBJECT_ID}`)
  ).toBeInTheDocument();
  expect(getBlobMediaPayload.mock.calls.filter(([hash]) => hash === IMAGE_HASH)).toHaveLength(
    callsBeforeDisable
  );
});
