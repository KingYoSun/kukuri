import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';

import { createDesktopMockApi } from '@/mocks/desktopApiMock';
import { App } from '@/App';
import {
  createDeferred,
  getDetailPane,
  buildImagePost,
  buildVideoPost,
  installObjectUrlMocks,
  setViewportWidth,
} from './DesktopShellPage.testHelpers';
import type { BlobViewStatus } from '@/lib/api';
import { DEVELOPER_MODE_STORAGE_KEY } from '@/lib/developerMode';

beforeEach(() => {
  setViewportWidth(1024);
  window.history.replaceState(null, '', '/');
});

afterEach(() => {
  vi.useRealTimers();
});

test('timeline image stops loading and hides unavailable media after a null response in normal mode', async () => {
  window.localStorage.setItem(DEVELOPER_MODE_STORAGE_KEY, 'false');
  const payload = createDeferred<null>();
  const api = createDesktopMockApi({
    seedPosts: {
      'kukuri:topic:general': [
        buildImagePost({ content: 'available caption', content_status: 'Available' }),
      ],
    },
  });
  api.getBlobMediaPayload = async () => payload.promise;

  render(<App api={api} />);

  expect(await screen.findByTestId('media-skeleton-image-post')).toBeInTheDocument();
  act(() => payload.resolve(null));

  await waitFor(() => {
    expect(screen.queryByTestId('media-skeleton-image-post')).not.toBeInTheDocument();
  });
  expect(screen.queryByText('image/png')).not.toBeInTheDocument();
  expect(screen.queryByText('Media unavailable.')).not.toBeInTheDocument();
});

test('timeline image stops loading and hides unavailable media after a rejected response in normal mode', async () => {
  window.localStorage.setItem(DEVELOPER_MODE_STORAGE_KEY, 'false');
  const payload = createDeferred<null>();
  const api = createDesktopMockApi({
    seedPosts: {
      'kukuri:topic:general': [
        buildImagePost({ content: 'available caption', content_status: 'Available' }),
      ],
    },
  });
  api.getBlobMediaPayload = async () => payload.promise;

  render(<App api={api} />);

  expect(await screen.findByTestId('media-skeleton-image-post')).toBeInTheDocument();
  act(() => payload.reject(new Error('blob unavailable')));

  await waitFor(() => {
    expect(screen.queryByTestId('media-skeleton-image-post')).not.toBeInTheDocument();
  });
  expect(screen.queryByText('image/png')).not.toBeInTheDocument();
  expect(screen.queryByText('blob unavailable')).not.toBeInTheDocument();
});

test('missing text body does not occupy normal UI', async () => {
  window.localStorage.setItem(DEVELOPER_MODE_STORAGE_KEY, 'false');
  render(
    <App
      api={createDesktopMockApi({
        seedPosts: {
          'kukuri:topic:general': [buildImagePost({ attachments: [] })],
        },
      })}
    />
  );

  await waitFor(() => {
    expect(document.querySelector('[data-post-object-id="image-post"]')).toBeInTheDocument();
  });
  expect(screen.queryByText('envelope-image-post')).not.toBeInTheDocument();
  expect(screen.queryByTestId('text-skeleton-image-post')).not.toBeInTheDocument();
  expect(screen.queryByText('[blob pending]')).not.toBeInTheDocument();
  expect(screen.queryByText('Content unavailable.')).not.toBeInTheDocument();
});

test('developer mode shows concise diagnostics after body and media become unavailable', async () => {
  const api = createDesktopMockApi({
    seedPosts: {
      'kukuri:topic:general': [buildImagePost()],
    },
  });
  api.getBlobMediaPayload = async () => null;

  render(<App api={api} />);

  expect(await screen.findByText('Content unavailable.')).toBeInTheDocument();
  expect(await screen.findByText('Media unavailable.')).toBeInTheDocument();
  expect(screen.queryByTestId('text-skeleton-image-post')).not.toBeInTheDocument();
  expect(screen.queryByTestId('media-skeleton-image-post')).not.toBeInTheDocument();
  expect(screen.queryByText('[blob pending]')).not.toBeInTheDocument();
});

test('developer mode replaces a missing image skeleton with a diagnostic', async () => {
  const api = createDesktopMockApi({
    seedPosts: {
      'kukuri:topic:general': [buildImagePost()],
    },
  });
  api.getBlobMediaPayload = async () => null;

  render(<App api={api} />);

  expect(await screen.findByText('Media unavailable.')).toBeInTheDocument();
  expect(screen.queryByTestId('media-skeleton-image-post')).not.toBeInTheDocument();
  expect(screen.queryByText('image/png')).not.toBeInTheDocument();
});

test('timeline image post switches to ready state when attachment becomes available', async () => {
  const missingPost = buildImagePost();
  const { rerender } = render(
    <App
      api={createDesktopMockApi({
        seedPosts: {
          'kukuri:topic:general': [missingPost],
        },
      })}
    />
  );

  await waitFor(() => {
    expect(screen.getByTestId('media-skeleton-image-post')).toBeInTheDocument();
  });

  rerender(
    <App
      api={createDesktopMockApi({
        seedPosts: {
          'kukuri:topic:general': [
            buildImagePost({
              content: 'caption ready',
              content_status: 'Available' satisfies BlobViewStatus,
              attachments: [
                {
                  ...missingPost.attachments[0],
                  status: 'Available',
                },
              ],
            }),
          ],
        },
      })}
    />
  );

  await waitFor(() => {
    expect(screen.getByText('caption ready')).toBeInTheDocument();
  });
  expect(screen.queryByTestId('media-skeleton-image-post')).not.toBeInTheDocument();
});

test('timeline image recovers when an existing refresh retries a previously unavailable hash', async () => {
  window.localStorage.setItem(DEVELOPER_MODE_STORAGE_KEY, 'false');
  installObjectUrlMocks();
  const missingPost = buildImagePost({ content: 'caption', content_status: 'Available' });
  const unavailableApi = createDesktopMockApi({
    seedPosts: { 'kukuri:topic:general': [missingPost] },
  });
  unavailableApi.getBlobMediaPayload = async () => null;
  const recoveredApi = createDesktopMockApi({
    seedPosts: {
      'kukuri:topic:general': [
        buildImagePost({
          content: 'caption',
          content_status: 'Available',
          attachments: [{ ...missingPost.attachments[0], status: 'Available' }],
        }),
      ],
    },
  });
  recoveredApi.getBlobMediaPayload = async (_hash, mime) => ({
    bytes_base64: 'ZmFrZS1pbWFnZQ==',
    mime,
  });

  const { rerender } = render(<App api={unavailableApi} />);
  await waitFor(() => {
    expect(screen.queryByTestId('media-skeleton-image-post')).not.toBeInTheDocument();
  });

  rerender(<App api={recoveredApi} />);

  expect(await screen.findByTestId('media-preview-image-post')).toHaveAttribute(
    'src',
    expect.stringContaining('blob:mock-')
  );
});

test('timeline image post renders actual preview when object-url payload is available', async () => {
  const { revokeObjectUrl } = installObjectUrlMocks();
  const api = createDesktopMockApi({
    seedPosts: {
      'kukuri:topic:general': [
        buildImagePost({
          content: 'caption ready',
          content_status: 'Available',
          attachments: [
            {
              hash: 'b'.repeat(64),
              mime: 'image/png',
              bytes: 4096,
              role: 'image_original',
              status: 'Available',
            },
          ],
        }),
      ],
    },
  });
  api.getBlobMediaPayload = async () => ({
    bytes_base64: 'ZmFrZS1pbWFnZQ==',
    mime: 'image/png',
  });

  const { unmount } = render(<App api={api} />);

  const preview = await screen.findByTestId('media-preview-image-post');
  expect(preview).toBeInTheDocument();
  const previewUrl = preview.getAttribute('src');
  expect(previewUrl).toMatch(/^blob:mock-\d+$/);
  expect(revokeObjectUrl).not.toHaveBeenCalledWith(previewUrl);

  unmount();
  expect(revokeObjectUrl).toHaveBeenCalledWith(previewUrl);
});

test('thread pane reuses the same unavailable media renderer', async () => {
  const user = userEvent.setup();
  const api = createDesktopMockApi({
    seedPosts: {
      'kukuri:topic:general': [
        buildImagePost(),
        {
          ...buildImagePost({
            object_id: 'reply-post',
            envelope_id: 'envelope-reply-post',
            object_kind: 'comment',
            content: 'reply body',
            content_status: 'Available',
            attachments: [],
            reply_to: 'image-post',
            root_id: 'image-post',
          }),
        },
      ],
    },
  });
  api.getBlobMediaPayload = async () => null;

  render(
    <App api={api} />
  );

  let imagePost: HTMLElement | null = null;
  await waitFor(() => {
    imagePost = document.querySelector(
      '[data-post-object-id="image-post"] [data-testid="post-identifier-target"]'
    );
    expect(imagePost).toBeInTheDocument();
  });
  expect(screen.queryByText('envelope-image-post')).not.toBeInTheDocument();
  await user.click(imagePost!);
  await waitFor(() => expect(getDetailPane('Thread')).toBeInTheDocument());
  const threadPanel = getDetailPane('Thread');

  expect(await within(threadPanel).findByText('Media unavailable.')).toBeInTheDocument();
  expect(within(threadPanel).queryByTestId('media-skeleton-image-post')).not.toBeInTheDocument();
});

test('developer mode reports an unavailable text body without rendering its placeholder', async () => {
  render(
    <App
      api={createDesktopMockApi({
        seedPosts: {
          'kukuri:topic:general': [buildImagePost({ attachments: [] })],
        },
      })}
    />
  );

  expect(await screen.findByText('Content unavailable.')).toBeInTheDocument();
  expect(screen.queryByTestId('text-skeleton-image-post')).not.toBeInTheDocument();
  expect(screen.queryByText('[blob pending]')).not.toBeInTheDocument();
});

test('developer mode replaces an unavailable video skeleton with a diagnostic', async () => {
  const api = createDesktopMockApi({
    seedPosts: {
      'kukuri:topic:general': [buildVideoPost()],
    },
  });
  api.getBlobMediaPayload = async () => null;

  render(<App api={api} />);

  expect(await screen.findByText('Media unavailable.')).toBeInTheDocument();
  expect(screen.queryByTestId('media-skeleton-video-post')).not.toBeInTheDocument();
  expect(screen.queryByText('video/mp4')).not.toBeInTheDocument();
});

test('poster-only video card renders poster preview without video element', async () => {
  installObjectUrlMocks();
  const api = createDesktopMockApi({
    seedPosts: {
      'kukuri:topic:general': [
        buildVideoPost({
          attachments: [
            {
              hash: 'v'.repeat(64),
              mime: 'video/mp4',
              bytes: 8192,
              role: 'video_manifest',
              status: 'Missing',
            },
            {
              hash: 'p'.repeat(64),
              mime: 'image/jpeg',
              bytes: 1024,
              role: 'video_poster',
              status: 'Available',
            },
          ],
        }),
      ],
    },
  });
  api.getBlobMediaPayload = async (hash, mime) =>
    hash === 'p'.repeat(64)
      ? {
          bytes_base64: 'ZmFrZS1wb3N0ZXI=',
          mime,
        }
      : null;

  render(<App api={api} />);

  const posterPreview = await screen.findByTestId('media-preview-video-post');
  expect(posterPreview).toBeInTheDocument();
  expect(screen.queryByTestId('media-video-video-post')).not.toBeInTheDocument();
  expect(posterPreview.getAttribute('src')).toContain('blob:mock-');
});

test('video card fetches manifest payload even when attachment status is missing', async () => {
  installObjectUrlMocks();
  const api = createDesktopMockApi({
    seedPosts: {
      'kukuri:topic:general': [
        buildVideoPost({
          attachments: [
            {
              hash: 'late-manifest'.repeat(4),
              mime: 'video/mp4',
              bytes: 9999,
              role: 'video_manifest',
              status: 'Missing',
            },
            {
              hash: 'late-poster'.repeat(4),
              mime: 'image/jpeg',
              bytes: 1024,
              role: 'video_poster',
              status: 'Available',
            },
          ],
        }),
      ],
    },
  });
  api.getBlobMediaPayload = async (hash, mime) => {
    if (hash === 'late-manifest'.repeat(4)) {
      return {
        bytes_base64: 'ZmFrZS12aWRlbw==',
        mime,
      };
    }
    if (hash === 'late-poster'.repeat(4)) {
      return {
        bytes_base64: 'ZmFrZS1wb3N0ZXI=',
        mime,
      };
    }
    return null;
  };

  render(<App api={api} />);

  const video = await screen.findByTestId('media-video-video-post');
  expect(video).toBeInTheDocument();
  expect(video).toHaveAttribute('src', expect.stringContaining('blob:mock-'));
});

test('video card retries after stalled manifest fetch after rerender', async () => {
  installObjectUrlMocks();
  const manifestHash = 'retry-manifest'.repeat(4);
  const posterHash = 'retry-poster'.repeat(4);
  const seedPosts = {
    'kukuri:topic:general': [
      buildVideoPost({
        attachments: [
          {
            hash: manifestHash,
            mime: 'video/mp4',
            bytes: 9999,
            role: 'video_manifest',
            status: 'Missing',
          },
          {
            hash: posterHash,
            mime: 'image/jpeg',
            bytes: 1024,
            role: 'video_poster',
            status: 'Missing',
          },
        ],
      }),
    ],
  };
  const stalledApi = createDesktopMockApi({
    seedPosts,
  });
  stalledApi.getBlobMediaPayload = async (hash, mime) => {
    if (hash === manifestHash) {
      return new Promise<null>(() => {});
    }
    if (hash === posterHash) {
      return {
        bytes_base64: 'ZmFrZS1wb3N0ZXI=',
        mime,
      };
    }
    return null;
  };
  const recoveredApi = createDesktopMockApi({
    seedPosts: {
      ...seedPosts,
    },
  });
  recoveredApi.getBlobMediaPayload = async (hash, mime) => {
    if (hash === manifestHash) {
      return {
        bytes_base64: 'ZmFrZS12aWRlbw==',
        mime,
      };
    }
    if (hash === posterHash) {
      return {
        bytes_base64: 'ZmFrZS1wb3N0ZXI=',
        mime,
      };
    }
    return null;
  };

  const { rerender } = render(<App api={stalledApi} />);

  await waitFor(() => {
    expect(screen.getByTestId('media-preview-video-post')).toBeInTheDocument();
  });

  rerender(<App api={recoveredApi} />);

  const video = await screen.findByTestId('media-video-video-post');
  expect(video).toBeInTheDocument();
  expect(video).toHaveAttribute('src', expect.stringContaining('blob:mock-'));
});

test('video card renders object-url playback source when manifest payload is available', async () => {
  installObjectUrlMocks();
  const api = createDesktopMockApi({
    seedPosts: {
      'kukuri:topic:general': [
        buildVideoPost({
          attachments: [
            {
              hash: 'manifest'.repeat(8),
              mime: 'video/mp4',
              bytes: 9999,
              role: 'video_manifest',
              status: 'Available',
            },
            {
              hash: 'poster'.repeat(8),
              mime: 'image/jpeg',
              bytes: 1024,
              role: 'video_poster',
              status: 'Available',
            },
          ],
        }),
      ],
    },
  });
  api.getBlobMediaPayload = async (hash, mime) => {
    if (hash === 'manifest'.repeat(8)) {
      return {
        bytes_base64: 'ZmFrZS12aWRlbw==',
        mime,
      };
    }
    if (hash === 'poster'.repeat(8)) {
      return {
        bytes_base64: 'ZmFrZS1wb3N0ZXI=',
        mime,
      };
    }
    return null;
  };

  render(<App api={api} />);

  const video = await screen.findByTestId('media-video-video-post');
  expect(video).toBeInTheDocument();
  expect(video.getAttribute('src')).toContain('blob:mock-');
});

test('video card falls back to poster preview when playback is unsupported on this client', async () => {
  installObjectUrlMocks();
  const api = createDesktopMockApi({
    seedPosts: {
      'kukuri:topic:general': [
        buildVideoPost({
          attachments: [
            {
              hash: 'manifest'.repeat(8),
              mime: 'video/mp4',
              bytes: 9999,
              role: 'video_manifest',
              status: 'Available',
            },
            {
              hash: 'poster'.repeat(8),
              mime: 'image/jpeg',
              bytes: 1024,
              role: 'video_poster',
              status: 'Available',
            },
          ],
        }),
      ],
    },
  });
  api.getBlobMediaPayload = async (hash, mime) => {
    if (hash === 'manifest'.repeat(8)) {
      return {
        bytes_base64: 'ZmFrZS12aWRlbw==',
        mime,
      };
    }
    if (hash === 'poster'.repeat(8)) {
      return {
        bytes_base64: 'ZmFrZS1wb3N0ZXI=',
        mime,
      };
    }
    return null;
  };

  render(<App api={api} />);

  const video = await screen.findByTestId('media-video-video-post');
  Object.defineProperty(video, 'error', {
    configurable: true,
    get: () => ({ code: 4 }),
  });
  fireEvent.error(video);

  await waitFor(() => {
    expect(screen.queryByTestId('media-video-video-post')).not.toBeInTheDocument();
  });
  expect(screen.getByTestId('media-preview-video-post')).toBeInTheDocument();
  expect(screen.getByAltText('video poster')).toBeInTheDocument();
});

