import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, expect, test, vi } from 'vitest';

import { App } from '@/App';
import { createDesktopMockApi } from '@/mocks/desktopApiMock';
import type { AuthorTrustGateResult, DesktopApi, PostView } from '@/lib/api';

import {
  buildPaginatedPost,
  getActiveColumn,
  setViewportWidth,
} from './DesktopShellPage.testHelpers';

// #1061 / ADR 0026 §8.4: 採用 CN の信頼値で非表示推奨の著者は、投稿を折りたたんで理由と
// 採用ノードを示し、この投稿だけを表示できる。判断は mute / block を変えない。

const NODE_BASE_URL = 'https://api.kukuri.app';
const HIDDEN_AUTHOR = 'b'.repeat(64);

function gateResult(hidden: string[]): (request: {
  author_pubkeys: string[];
}) => Promise<AuthorTrustGateResult> {
  return async ({ author_pubkeys }) => ({
    gates: author_pubkeys.map((author_pubkey) => ({
      author_pubkey,
      hidden: hidden.includes(author_pubkey),
      node_base_url: hidden.includes(author_pubkey) ? NODE_BASE_URL : null,
      reasons: hidden.includes(author_pubkey)
        ? (['related_users_block_or_mute'] as const).slice()
        : [],
      expires_at: null,
      always_visible: false,
    })),
  });
}

function hiddenPost(overrides?: Partial<PostView>): PostView {
  return buildPaginatedPost(1, {
    object_id: 'gated-post',
    root_id: 'gated-post',
    content: 'post from a low trust author',
    author_pubkey: HIDDEN_AUTHOR,
    content_status: 'Available',
    ...overrides,
  });
}

async function createGatedApi(posts: PostView[], hidden = [HIDDEN_AUTHOR]): Promise<DesktopApi> {
  const api = createDesktopMockApi({ seedPosts: { 'kukuri:topic:general': posts } });
  // 採用順位を設定した状態から始める（未設定では照会しない）。
  await api.setCommunityNodeConfig([{ base_url: NODE_BASE_URL }], [NODE_BASE_URL]);
  vi.spyOn(api, 'evaluateAuthorTrustGates').mockImplementation(gateResult(hidden));
  return api;
}

beforeEach(() => {
  setViewportWidth(1024);
  window.history.replaceState(null, '', '/');
});

test('a low trust author is collapsed on the timeline and can be revealed for this post', async () => {
  const user = userEvent.setup();
  const api = await createGatedApi([hiddenPost()]);
  const muteAuthor = vi.spyOn(api, 'muteAuthor');
  const blockAuthor = vi.spyOn(api, 'blockAuthor');

  render(<App api={api} />);

  const column = getActiveColumn('Timeline');
  const notice = await within(column).findByTestId('author-trust-gate-notice');
  expect(notice).toHaveTextContent(/collapsed|折りたたんで/);
  expect(notice).toHaveTextContent(NODE_BASE_URL);
  expect(screen.queryByText('post from a low trust author')).not.toBeInTheDocument();

  // 「表示する」はこの投稿だけに効き、mute / block も作らない。
  await user.click(within(notice).getByTestId('author-trust-gate-reveal'));
  expect(await screen.findByText('post from a low trust author')).toBeInTheDocument();
  expect(muteAuthor).not.toHaveBeenCalled();
  expect(blockAuthor).not.toHaveBeenCalled();
});

test('a repost of a low trust author is collapsed by the source author', async () => {
  const api = await createGatedApi([
    hiddenPost({
      object_id: 'repost-of-gated',
      root_id: 'repost-of-gated',
      author_pubkey: 'c'.repeat(64),
      content: 'repost body',
      repost_of: {
        source_object_id: 'gated-source',
        source_topic_id: 'kukuri:topic:general',
        source_author_pubkey: HIDDEN_AUTHOR,
        source_object_kind: 'post',
        content: 'source body',
        attachments: [],
      },
    }),
  ]);

  render(<App api={api} />);

  const column = getActiveColumn('Timeline');
  expect(await within(column).findByTestId('author-trust-gate-notice')).toHaveTextContent(
    /person quoted|引用元/
  );
  expect(screen.queryByText('source body')).not.toBeInTheDocument();
});

test('no node is queried and nothing is collapsed without an adopted priority', async () => {
  const api = createDesktopMockApi({ seedPosts: { 'kukuri:topic:general': [hiddenPost()] } });
  const evaluate = vi.spyOn(api, 'evaluateAuthorTrustGates');

  render(<App api={api} />);

  expect(await screen.findByText('post from a low trust author')).toBeInTheDocument();
  await waitFor(() => expect(screen.queryByTestId('author-trust-gate-notice')).not.toBeInTheDocument());
  expect(evaluate).not.toHaveBeenCalled();
});

test('the author detail exception keeps a low trust author visible', async () => {
  const user = userEvent.setup();
  const api = await createGatedApi([hiddenPost()], []);
  const setException = vi.spyOn(api, 'setAuthorTrustDisplayException');

  render(<App api={api} />);

  // 折りたたまれた投稿の案内から作者詳細へ移動できる（管理導線への到達）。
  const api2 = api;
  vi.spyOn(api2, 'evaluateAuthorTrustGates').mockImplementation(gateResult([HIDDEN_AUTHOR]));
  const column = getActiveColumn('Timeline');
  const notice = await within(column).findByTestId('author-trust-gate-notice');
  await user.click(within(notice).getByTestId('author-trust-gate-open-author'));
  const toggle = await screen.findByTestId('author-trust-display-exception-toggle');
  expect(toggle).not.toBeChecked();
  await user.click(toggle);
  await waitFor(() => expect(setException).toHaveBeenCalledWith(HIDDEN_AUTHOR, true));
  await waitFor(() => expect(toggle).toBeChecked());
});
