import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';

import { createDesktopMockApi } from '@/mocks/desktopApiMock';
import { App } from '@/App';
import {
  installFailedPosterGenerationMocks,
  installMetadataSeekPosterGenerationMocks,
  installObjectUrlMocks,
  installSuccessfulPosterGenerationMocks,
  openPublishDialog,
  setViewportWidth,
} from './DesktopShellPage.testHelpers';
import type { CreateAttachmentInput } from '@/lib/api';

beforeEach(() => {
  setViewportWidth(1024);
  window.history.replaceState(null, '', '/');
});

afterEach(() => {
  vi.useRealTimers();
});

test('single attach button classifies mixed image and video files', async () => {
  installObjectUrlMocks();
  installSuccessfulPosterGenerationMocks();
  let attachmentsSeen: CreateAttachmentInput[] = [];
  const api = createDesktopMockApi();
  const originalCreatePost = api.createPost;
  api.createPost = async (topic, content, replyTo, attachments) => {
    attachmentsSeen = attachments ?? [];
    return originalCreatePost(topic, content, replyTo, attachments);
  };

  const user = userEvent.setup();
  render(<App api={api} />);

  const publishDialog = await openPublishDialog(user);
  await user.upload(within(publishDialog).getByLabelText(/attachment/i), [
    new File([Uint8Array.from([1, 2, 3, 4])], 'flower.png', { type: 'image/png' }),
    new File([Uint8Array.from([5, 6, 7, 8])], 'clip.mp4', { type: 'video/mp4' }),
  ]);
  await waitFor(() => {
    expect(screen.getByText('flower.png')).toBeInTheDocument();
    expect(screen.getByText('clip.mp4')).toBeInTheDocument();
  });
  await user.click(within(publishDialog).getByRole('button', { name: 'Post' }));

  await waitFor(() => {
    expect(attachmentsSeen).toHaveLength(3);
  });
  expect(attachmentsSeen.map((attachment) => attachment.role)).toEqual([
    'image_original',
    'video_manifest',
    'video_poster',
  ]);
});

test('video upload generates poster attachment before publish', async () => {
  installObjectUrlMocks();
  installSuccessfulPosterGenerationMocks();
  let attachmentsSeen: CreateAttachmentInput[] = [];
  const api = createDesktopMockApi();
  const originalCreatePost = api.createPost;
  api.createPost = async (topic, content, replyTo, attachments) => {
    attachmentsSeen = attachments ?? [];
    return originalCreatePost(topic, content, replyTo, attachments);
  };

  const user = userEvent.setup();
  render(<App api={api} />);

  const publishDialog = await openPublishDialog(user);
  await user.upload(
    within(publishDialog).getByLabelText(/attachment/i),
    new File([Uint8Array.from([7, 8, 9])], 'clip.mp4', { type: 'video/mp4' })
  );

  await waitFor(() => {
    expect(screen.getByText(/video_manifest/)).toBeInTheDocument();
  });
  expect(screen.getByText(/video_poster/)).toBeInTheDocument();

  await user.click(within(publishDialog).getByRole('button', { name: 'Post' }));

  await waitFor(() => {
    expect(attachmentsSeen).toHaveLength(2);
  });
  expect(attachmentsSeen.some((attachment) => attachment.role === 'video_manifest')).toBe(true);
  expect(attachmentsSeen.some((attachment) => attachment.role === 'video_poster')).toBe(true);
});

test('video upload generates poster attachment with metadata seek fallback', async () => {
  installObjectUrlMocks();
  installMetadataSeekPosterGenerationMocks();
  let attachmentsSeen: CreateAttachmentInput[] = [];
  const api = createDesktopMockApi();
  const originalCreatePost = api.createPost;
  api.createPost = async (topic, content, replyTo, attachments) => {
    attachmentsSeen = attachments ?? [];
    return originalCreatePost(topic, content, replyTo, attachments);
  };

  const user = userEvent.setup();
  render(<App api={api} />);

  const publishDialog = await openPublishDialog(user);
  await user.upload(
    within(publishDialog).getByLabelText(/attachment/i),
    new File([Uint8Array.from([7, 8, 9])], 'clip.mp4', { type: 'video/mp4' })
  );

  await waitFor(() => {
    expect(screen.getByText(/video_manifest/)).toBeInTheDocument();
  });
  expect(screen.getByText(/video_poster/)).toBeInTheDocument();

  await user.click(within(publishDialog).getByRole('button', { name: 'Post' }));

  await waitFor(() => {
    expect(attachmentsSeen).toHaveLength(2);
  });
  expect(attachmentsSeen.some((attachment) => attachment.role === 'video_poster')).toBe(true);
});

test('video poster generation failure blocks publish', async () => {
  installObjectUrlMocks();
  installFailedPosterGenerationMocks();
  const api = createDesktopMockApi();
  const createPostSpy = vi.fn(api.createPost);
  api.createPost = createPostSpy;

  const user = userEvent.setup();
  render(<App api={api} />);

  const publishDialog = await openPublishDialog(user);
  await user.upload(
    within(publishDialog).getByLabelText(/attachment/i),
    new File([Uint8Array.from([1, 3, 5, 7])], 'broken.mp4', { type: 'video/mp4' })
  );

  await waitFor(() => {
    expect(screen.getAllByText('Failed to generate the video preview.').length).toBeGreaterThan(0);
  });

  await user.click(within(publishDialog).getByRole('button', { name: 'Post' }));

  expect(createPostSpy).not.toHaveBeenCalled();
});

test('composer shows image draft preview before publish', async () => {
  installObjectUrlMocks();
  const user = userEvent.setup();
  render(<App api={createDesktopMockApi()} />);

  const publishDialog = await openPublishDialog(user);
  await user.upload(
    within(publishDialog).getByLabelText(/attachment/i),
    new File([Uint8Array.from([1, 2, 3, 4])], 'flower.png', { type: 'image/png' })
  );

  expect(await within(publishDialog).findByRole('img', { name: 'draft preview flower.png' })).toBeInTheDocument();
  expect(within(publishDialog).getByText(/image_original/)).toBeInTheDocument();
  expect(within(publishDialog).getByText(/image\/png/)).toBeInTheDocument();
  expect(within(publishDialog).getByText(/4 B/)).toBeInTheDocument();
});

test('composer shows video poster draft preview before publish', async () => {
  installObjectUrlMocks();
  installSuccessfulPosterGenerationMocks();
  const user = userEvent.setup();
  render(<App api={createDesktopMockApi()} />);

  const publishDialog = await openPublishDialog(user);
  await user.upload(
    within(publishDialog).getByLabelText(/attachment/i),
    new File([Uint8Array.from([7, 8, 9])], 'clip.mp4', { type: 'video/mp4' })
  );

  expect(await within(publishDialog).findByRole('img', { name: 'draft preview clip.mp4' })).toBeInTheDocument();
  expect(within(publishDialog).getByText(/video_manifest/)).toBeInTheDocument();
  expect(within(publishDialog).getByText(/video_poster/)).toBeInTheDocument();
  expect(within(publishDialog).getByText(/image\/jpeg/)).toBeInTheDocument();
});

