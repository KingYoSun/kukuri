import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, expect, test, vi } from 'vitest';

import { createDesktopMockApi } from '@/mocks/desktopApiMock';
import { App } from '@/App';
import {
  buildImagePost,
  openSettingsSection,
  setViewportWidth,
} from './DesktopShellPage.testHelpers';

// #858 / ADR 0046: 表示許可前は、成人向けラベル付き投稿の添付メディアへの
// ネットワークリクエスト(getBlobMediaPayload)とデコードが発生しないこと、
// テキストは折りたたみの代替表示になることを固定する。テキストとメディアで
// 検証を分ける(issue の受入条件)。

const ADULT_HASH = 'a'.repeat(64);

function buildAdultImagePost() {
  return buildImagePost({
    object_id: 'adult-image-post',
    content: 'labeled adult caption',
    content_status: 'Available',
    content_labels: ['adult'],
  });
}

beforeEach(() => {
  setViewportWidth(1024);
  window.history.replaceState(null, '', '/');
});

test('adult-labeled media is not requested and shows a placeholder while display is off', async () => {
  const api = createDesktopMockApi({
    seedPosts: {
      'kukuri:topic:general': [buildAdultImagePost()],
    },
  });
  const getBlobMediaPayload = vi.fn(api.getBlobMediaPayload);
  api.getBlobMediaPayload = getBlobMediaPayload;

  render(<App api={api} />);

  // メディア: 一貫したプレースホルダー表示。
  expect(await screen.findByTestId('media-adult-gated-adult-image-post')).toBeInTheDocument();
  // テキスト: 本文の代わりに代替表示。
  expect(screen.getByTestId('post-adult-gated-adult-image-post')).toBeInTheDocument();
  expect(screen.queryByText('labeled adult caption')).not.toBeInTheDocument();

  // 取得制御: 対象 hash への取得リクエストが 1 度も発生しない。
  await waitFor(() => {
    expect(
      getBlobMediaPayload.mock.calls.filter(([hash]) => hash === ADULT_HASH)
    ).toHaveLength(0);
  });
});

test('adult-labeled text is hidden independently of media fetch control', async () => {
  // メディアを持たない成人向けテキスト投稿でも、本文が代替表示になる。
  const api = createDesktopMockApi({
    seedPosts: {
      'kukuri:topic:general': [
        buildImagePost({
          object_id: 'adult-text-post',
          content: 'text only adult body',
          content_status: 'Available',
          attachments: [],
          content_labels: ['adult'],
        }),
      ],
    },
  });

  render(<App api={api} />);

  expect(await screen.findByTestId('post-adult-gated-adult-text-post')).toBeInTheDocument();
  expect(screen.queryByText('text only adult body')).not.toBeInTheDocument();
});

test('adult-labeled quote source gates the enclosing card and media fetch', async () => {
  const post = buildImagePost({
    object_id: 'adult-quote-source-host',
    content: 'safe-looking quote commentary',
    content_labels: [],
    object_kind: 'repost',
    repost_commentary: 'safe-looking quote commentary',
    repost_of: {
      source_object_id: 'adult-quote-source',
      source_topic_id: 'kukuri:topic:general',
      source_author_pubkey: 'b'.repeat(64),
      source_author_name: 'source-author',
      source_author_display_name: 'Source Author',
      source_author_picture: null,
      source_author_picture_asset: null,
      source_object_kind: 'post',
      content: 'adult quote source body',
      attachments: [],
      content_labels: ['adult'],
      reply_to: null,
      root_id: 'adult-quote-source',
    },
  });
  const api = createDesktopMockApi({ seedPosts: { 'kukuri:topic:general': [post] } });
  const getBlobMediaPayload = vi.fn(api.getBlobMediaPayload);
  api.getBlobMediaPayload = getBlobMediaPayload;

  render(<App api={api} />);

  expect(
    await screen.findByTestId('post-adult-gated-adult-quote-source-host')
  ).toBeInTheDocument();
  expect(screen.queryByText('safe-looking quote commentary')).not.toBeInTheDocument();
  expect(screen.queryByText('adult quote source body')).not.toBeInTheDocument();
  await waitFor(() => {
    expect(getBlobMediaPayload.mock.calls.filter(([hash]) => hash === ADULT_HASH)).toHaveLength(0);
  });
});

test('adult-labeled reply preview gates the enclosing card and media fetch', async () => {
  const post = buildImagePost({
    object_id: 'adult-reply-preview-host',
    content: 'safe-looking reply body',
    content_labels: [],
    reply_to: 'adult-reply-parent',
    reply_preview: {
      object_id: 'adult-reply-parent',
      topic: 'kukuri:topic:general',
      author: {
        pubkey: 'b'.repeat(64),
        name: 'parent-author',
        display_name: 'Parent Author',
        picture: null,
        picture_asset: null,
      },
      content: 'adult reply preview body',
      attachments: [],
      content_labels: ['adult'],
      root_id: 'adult-reply-parent',
      reply_to: null,
    },
  });
  const api = createDesktopMockApi({ seedPosts: { 'kukuri:topic:general': [post] } });
  const getBlobMediaPayload = vi.fn(api.getBlobMediaPayload);
  api.getBlobMediaPayload = getBlobMediaPayload;

  render(<App api={api} />);

  expect(
    await screen.findByTestId('post-adult-gated-adult-reply-preview-host')
  ).toBeInTheDocument();
  expect(screen.queryByText('safe-looking reply body')).not.toBeInTheDocument();
  expect(screen.queryByText('adult reply preview body')).not.toBeInTheDocument();
  await waitFor(() => {
    expect(getBlobMediaPayload.mock.calls.filter(([hash]) => hash === ADULT_HASH)).toHaveLength(0);
  });
});

test('unlabeled media keeps fetching while adult display is off', async () => {
  const api = createDesktopMockApi({
    seedPosts: {
      'kukuri:topic:general': [
        buildImagePost({ content: 'plain caption', content_status: 'Available' }),
      ],
    },
  });
  const getBlobMediaPayload = vi.fn(api.getBlobMediaPayload);
  api.getBlobMediaPayload = getBlobMediaPayload;

  render(<App api={api} />);

  await waitFor(() => {
    expect(
      getBlobMediaPayload.mock.calls.some(([hash]) => hash === ADULT_HASH)
    ).toBe(true);
  });
  expect(screen.queryByTestId('media-adult-gated-image-post')).not.toBeInTheDocument();
});

test('enabling the safety setting fetches adult media and disabling stops and clears it', async () => {
  const user = userEvent.setup();
  const api = createDesktopMockApi({
    seedPosts: {
      'kukuri:topic:general': [buildAdultImagePost()],
    },
  });
  const getBlobMediaPayload = vi.fn(api.getBlobMediaPayload);
  api.getBlobMediaPayload = getBlobMediaPayload;
  const setAdultContentDisplayEnabled = vi.fn(api.setAdultContentDisplayEnabled);
  api.setAdultContentDisplayEnabled = setAdultContentDisplayEnabled;

  render(<App api={api} />);

  expect(await screen.findByTestId('media-adult-gated-adult-image-post')).toBeInTheDocument();
  expect(getBlobMediaPayload.mock.calls.filter(([hash]) => hash === ADULT_HASH)).toHaveLength(0);

  // 設定画面から明示的に有効化する。
  await openSettingsSection(user, 'safety');
  await user.click(screen.getByTestId('adult-content-display-toggle'));

  await waitFor(() => {
    expect(setAdultContentDisplayEnabled).toHaveBeenCalledWith(true);
  });
  await waitFor(() => {
    expect(
      getBlobMediaPayload.mock.calls.some(([hash]) => hash === ADULT_HASH)
    ).toBe(true);
  });
  await waitFor(() => {
    expect(
      screen.queryByTestId('media-adult-gated-adult-image-post')
    ).not.toBeInTheDocument();
  });

  // OFF へ戻すと以後の取得が止まり、表示済みメディアも破棄される。
  const callsBeforeDisable = getBlobMediaPayload.mock.calls.filter(
    ([hash]) => hash === ADULT_HASH
  ).length;
  await user.click(screen.getByTestId('adult-content-display-toggle'));
  await waitFor(() => {
    expect(setAdultContentDisplayEnabled).toHaveBeenCalledWith(false);
  });
  expect(await screen.findByTestId('media-adult-gated-adult-image-post')).toBeInTheDocument();
  expect(
    getBlobMediaPayload.mock.calls.filter(([hash]) => hash === ADULT_HASH)
  ).toHaveLength(callsBeforeDisable);
});
