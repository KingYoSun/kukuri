import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, expect, test, vi } from 'vitest';

import { createDesktopMockApi } from '@/mocks/desktopApiMock';
import { App } from '@/App';
import type {
  BlobMediaPayload,
  CommunityNodeContentAdvisoryLookupResult,
  ContentAdvisory,
  DesktopApi,
  PostView,
} from '@/lib/api';

import {
  buildImagePost,
  getActiveColumn,
  installObjectUrlMocks,
  openSettingsSection,
  setViewportWidth,
} from './DesktopShellPage.testHelpers';

// #1107: 成人向け表示の切替と、表示済みメディア・advisory 照会・取得ゲートの状態遷移。

const SHARED_HASH = 'c'.repeat(64);
const NODE_BASE_URL = 'https://api.kukuri.app';
const ISSUER_NODE_ID = 'd'.repeat(64);

function postAdvisory(postId: string): ContentAdvisory {
  return {
    issuer_node_id: ISSUER_NODE_ID,
    subject_kind: 'post_id',
    subject_id: postId,
    category: 'nsfw',
    label: 'adult',
    confidence: 84,
    signal_id: 'signal-1107',
    basis: 'classifier_score',
  };
}

function imagePost(objectId: string, hash: string, createdAt: number): PostView {
  return buildImagePost({
    object_id: objectId,
    root_id: objectId,
    content: `${objectId} caption`,
    content_status: 'Available',
    created_at: createdAt,
    attachments: [
      { hash, mime: 'image/png', bytes: 2048, role: 'image_original', status: 'Available' },
    ],
  });
}

function mockLookup(api: DesktopApi, advisories: ContentAdvisory[]) {
  return vi
    .spyOn(api, 'lookupCommunityNodeContentAdvisories')
    .mockImplementation(async (request) => {
      const requested = new Set([...request.post_ids, ...request.blob_hashes]);
      return {
        nodes: [
          {
            base_url: NODE_BASE_URL,
            node_id: ISSUER_NODE_ID,
            advisories: advisories.filter((advisory) => requested.has(advisory.subject_id)),
            error: null,
          },
        ],
      } satisfies CommunityNodeContentAdvisoryLookupResult;
    });
}

function requestsFor(mock: ReturnType<typeof vi.fn>, hash: string) {
  return mock.mock.calls.filter(([requested]) => requested === hash).length;
}

/// 表示中に一度でも media-preview が現れたかを記録する(ちらつきの検出)。
function observePreviews(testIds: string[]) {
  const seen = new Set<string>();
  const check = () => {
    for (const testId of testIds) {
      if (document.querySelector(`[data-testid="${testId}"]`)) seen.add(testId);
    }
  };
  const observer = new MutationObserver(check);
  observer.observe(document.body, { childList: true, subtree: true, attributes: true });
  return {
    seen,
    stop: () => observer.disconnect(),
  };
}

// 起動・照会の debounce・取得・設定操作を順に待つため、負荷の高い CI でも収まる上限にする。
const TEST_TIMEOUT_MS = 30_000;
const WAIT = { timeout: 10_000 };

beforeEach(() => {
  setViewportWidth(1024);
  window.history.replaceState(null, '', '/');
});

// AC-1 / AC-2 / AC-3 / AC-5 / AC-6: advisory 付き投稿と advisory なし投稿が同じ blob を参照する。
// OFF 切替後は両方のメディアを代替表示にし、表示・再取得を一度も起こさない。ON へ戻すと両方表示する。
test('turning adult display off gates a blob shared by advisory and plain posts without flicker', async () => {
  const user = userEvent.setup();
  installObjectUrlMocks();
  const api = createDesktopMockApi({
    seedPosts: {
      'kukuri:topic:general': [
        imagePost('advisory-shared-post', SHARED_HASH, 1),
        imagePost('plain-shared-post', SHARED_HASH, 2),
      ],
    },
  });
  await api.setAdultContentDisplayEnabled(true);
  mockLookup(api, [postAdvisory('advisory-shared-post')]);
  const getBlobMediaPayload = vi.fn(api.getBlobMediaPayload);
  api.getBlobMediaPayload = getBlobMediaPayload;

  render(<App api={api} />);

  const column = getActiveColumn('Timeline');
  expect(
    await within(column).findByTestId('media-preview-plain-shared-post', {}, WAIT)
  ).toBeInTheDocument();
  expect(within(column).getByTestId('media-preview-advisory-shared-post')).toBeInTheDocument();
  const requestsBeforeDisable = requestsFor(getBlobMediaPayload, SHARED_HASH);
  expect(requestsBeforeDisable).toBeGreaterThan(0);

  await openSettingsSection(user, 'safety');
  const toggle = screen.getByTestId('adult-content-display-toggle');
  await user.click(toggle);
  const previews = observePreviews([
    'media-preview-advisory-shared-post',
    'media-preview-plain-shared-post',
  ]);

  expect(
    await within(column).findByTestId('media-adult-gated-advisory-shared-post', {}, WAIT)
  ).toBeInTheDocument();
  expect(
    await within(column).findByTestId('media-adult-gated-plain-shared-post', {}, WAIT)
  ).toBeInTheDocument();
  // advisory の無い投稿は本文を伏せない(メディアだけを代替表示にする)。
  expect(within(column).getByText('plain-shared-post caption')).toBeInTheDocument();
  previews.seen.clear();
  await new Promise((resolve) => setTimeout(resolve, 800));
  previews.stop();
  expect([...previews.seen]).toEqual([]);
  expect(within(column).getByTestId('media-adult-gated-plain-shared-post')).toBeInTheDocument();
  expect(requestsFor(getBlobMediaPayload, SHARED_HASH)).toBe(requestsBeforeDisable);

  // AC-5: ON へ戻すと両方の投稿で表示される。
  await user.click(toggle);
  expect(
    await within(column).findByTestId('media-preview-plain-shared-post', {}, WAIT)
  ).toBeInTheDocument();
  expect(
    await within(column).findByTestId('media-preview-advisory-shared-post', {}, WAIT)
  ).toBeInTheDocument();
}, TEST_TIMEOUT_MS);

// AC-1 / AC-4: 照会完了で複数の通常画像の取得が同時に始まり、一方の完了で他方の結果を捨てない。
test('plain image posts all leave the skeleton when their fetches complete out of order', async () => {
  installObjectUrlMocks();
  const fastHash = 'e'.repeat(64);
  const slowHash = 'f'.repeat(64);
  const api = createDesktopMockApi({
    seedPosts: {
      'kukuri:topic:general': [
        imagePost('fast-image-post', fastHash, 1),
        imagePost('slow-image-post', slowHash, 2),
      ],
    },
  });
  mockLookup(api, []);
  const getBlobMediaPayload = vi.fn(
    async (hash: string, mime: string): Promise<BlobMediaPayload | null> => {
      if (hash === slowHash) {
        await new Promise((resolve) => setTimeout(resolve, 200));
      }
      return { bytes_base64: 'ZmFrZS1pbWFnZQ==', mime };
    }
  );
  api.getBlobMediaPayload = getBlobMediaPayload;

  render(<App api={api} />);

  const column = getActiveColumn('Timeline');
  expect(await within(column).findByTestId('media-preview-fast-image-post', {}, WAIT)).toBeInTheDocument();
  await waitFor(
    () => {
      expect(within(column).getByTestId('media-preview-slow-image-post')).toBeInTheDocument();
    },
    WAIT
  );
  expect(requestsFor(getBlobMediaPayload, slowHash)).toBe(1);
}, TEST_TIMEOUT_MS);

// INVAR-1 / AC-3: 取得中に OFF でゲート対象になった blob は、取得完了後も表示に使わない。
test('a fetch that completes after its blob became gated is discarded', async () => {
  const user = userEvent.setup();
  installObjectUrlMocks();
  const api = createDesktopMockApi({
    seedPosts: {
      'kukuri:topic:general': [imagePost('late-advisory-post', SHARED_HASH, 1)],
    },
  });
  await api.setAdultContentDisplayEnabled(true);
  mockLookup(api, [postAdvisory('late-advisory-post')]);
  let releaseFetch: (() => void) | null = null;
  const getBlobMediaPayload = vi.fn(
    async (hash: string, mime: string): Promise<BlobMediaPayload | null> => {
      await new Promise<void>((resolve) => {
        releaseFetch = resolve;
      });
      return { bytes_base64: 'ZmFrZS1pbWFnZQ==', mime };
    }
  );
  api.getBlobMediaPayload = getBlobMediaPayload;

  render(<App api={api} />);

  const column = getActiveColumn('Timeline');
  await waitFor(() => {
    expect(requestsFor(getBlobMediaPayload, SHARED_HASH)).toBe(1);
  }, WAIT);
  await openSettingsSection(user, 'safety');
  await user.click(screen.getByTestId('adult-content-display-toggle'));
  expect(
    await within(column).findByTestId('media-adult-gated-late-advisory-post', {}, WAIT)
  ).toBeInTheDocument();

  const release = releaseFetch as (() => void) | null;
  release?.();
  await new Promise((resolve) => setTimeout(resolve, 300));

  // ON へ戻したとき、破棄済みの結果ではなく新しい取得で表示する。
  await user.click(screen.getByTestId('adult-content-display-toggle'));
  await waitFor(() => {
    expect(requestsFor(getBlobMediaPayload, SHARED_HASH)).toBe(2);
  }, WAIT);
  expect(within(column).queryByTestId('media-preview-late-advisory-post')).not.toBeInTheDocument();
  (releaseFetch as (() => void) | null)?.();
  expect(
    await within(column).findByTestId('media-preview-late-advisory-post', {}, WAIT)
  ).toBeInTheDocument();
}, TEST_TIMEOUT_MS);
