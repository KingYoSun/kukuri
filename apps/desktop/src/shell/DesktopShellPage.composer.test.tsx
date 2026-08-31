import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';

import { createDesktopMockApi } from '@/mocks/desktopApiMock';
import { App } from '@/App';
import {
  closestSection,
  createDeferred,
  expectActiveTopic,
  openControlCenter,
  openPublishDialog,
  openSettingsSection,
  publishPost,
  setViewportWidth,
} from './DesktopShellPage.testHelpers';
import type { TimelineView } from '@/lib/api';

beforeEach(() => {
  setViewportWidth(1024);
  window.history.replaceState(null, '', '/');
});

afterEach(() => {
  vi.useRealTimers();
});

test('desktop shell can publish and render a post', async () => {
  const user = userEvent.setup();
  render(<App api={createDesktopMockApi()} />);

  await publishPost(user, 'hello desktop');

  await waitFor(() => {
    expect(screen.getByText('hello desktop')).toBeInTheDocument();
  });
  expectActiveTopic('kukuri:topic:general');
  expect(screen.queryByTestId('shell-nav-trigger')).not.toBeInTheDocument();
  const controlCenter = await openControlCenter(user);
  const generalTopic = within(controlCenter).getByRole('button', { name: 'general' }).closest('li');
  expect(generalTopic).not.toBeNull();
  expect(generalTopic).toHaveTextContent('joined / peers: 1');

  const drawer = await openSettingsSection(user, 'connectivity');
  expect(within(drawer).getByDisplayValue('peer1@127.0.0.1:7777')).toBeInTheDocument();
  const syncSection = closestSection(within(drawer).getByRole('heading', { name: 'Sync Status' }));
  expect(within(syncSection).getAllByText('Configured Peers').length).toBeGreaterThan(0);
  expect(within(syncSection).getByText('Connected to all configured peers')).toBeInTheDocument();
  expect(within(syncSection).getAllByText('peer-a').length).toBeGreaterThan(0);
});

test('desktop shell can enter reply mode and render reply state', async () => {
  const user = userEvent.setup();
  render(<App api={createDesktopMockApi()} />);

  await publishPost(user, 'root post');
  await waitFor(() => {
    expect(screen.getByText('root post')).toBeInTheDocument();
  });

  await user.click(screen.getAllByRole('button', { name: 'Reply' })[0]);
  const replyColumn = await screen.findByRole('region', { name: /Thread Column/ });
  expect(within(replyColumn).getByPlaceholderText('Write a reply')).toBeInTheDocument();
  expect(within(replyColumn).getByText('Replying')).toBeInTheDocument();
  expect(within(replyColumn).getAllByText('root post').length).toBeGreaterThan(0);

  const replyInput = within(replyColumn).getByPlaceholderText('Write a reply');
  await user.type(replyInput, 'reply post');
  const composer = replyInput.closest('form');
  if (!composer) {
    throw new Error('reply composer form not found');
  }
  await user.click(within(composer).getByRole('button', { name: 'Reply' }));

  await waitFor(() => {
    expect(screen.getAllByText('reply post').length).toBeGreaterThan(0);
  });
  expect(screen.getAllByRole('button', { name: 'Reply' }).length).toBeGreaterThan(0);
});

test('compose dialog stays width-safe when the source post contains a long token', async () => {
  const user = userEvent.setup();
  const api = createDesktopMockApi();
  const longContent = 'channel_payload_'.repeat(48);

  render(<App api={api} />);

  const publishDialog = await openPublishDialog(user);
  fireEvent.change(within(publishDialog).getByPlaceholderText('Write a post'), {
    target: { value: longContent },
  });
  await user.click(within(publishDialog).getByRole('button', { name: 'Post' }));
  await waitFor(() => {
    expect(screen.queryByRole('dialog', { name: 'Post' })).not.toBeInTheDocument();
  });
  await waitFor(() => {
    expect(screen.getByText(longContent)).toBeInTheDocument();
  });

  await user.click(screen.getAllByRole('button', { name: 'Reply' })[0]);
  const replyColumn = await screen.findByRole('region', { name: /Thread Column/ });

  expect(within(replyColumn).getAllByText(longContent)[0]).toHaveClass('post-copy-wrap');
});

test('reply publish reloads thread only once after a successful submit', async () => {
  const user = userEvent.setup();
  const api = createDesktopMockApi();
  const originalListThread = api.listThread;
  const listThreadSpy = vi.fn((topic, threadId, cursor, limit) =>
    originalListThread(topic, threadId, cursor, limit)
  );
  api.listThread = listThreadSpy;

  render(<App api={api} />);

  await publishPost(user, 'root post');
  await waitFor(() => {
    expect(screen.getByText('root post')).toBeInTheDocument();
  });

  await user.click(screen.getAllByRole('button', { name: 'Reply' })[0]);
  const replyColumn = await screen.findByRole('region', { name: /Thread Column/ });
  const threadCallsBeforeSubmit = listThreadSpy.mock.calls.length;

  const replyInput = within(replyColumn).getByPlaceholderText('Write a reply');
  await user.type(replyInput, 'reply post');
  const composer = replyInput.closest('form');
  if (!composer) {
    throw new Error('reply composer form not found');
  }
  await user.click(within(composer).getByRole('button', { name: 'Reply' }));

  await waitFor(() => {
    expect(screen.getAllByText('reply post').length).toBeGreaterThan(0);
  });
  expect(listThreadSpy.mock.calls.length - threadCallsBeforeSubmit).toBe(1);
});

test('publish dialog closes without waiting for a timeline refresh after submit', async () => {
  const api = createDesktopMockApi();
  const listTimelineDeferreds: Array<ReturnType<typeof createDeferred<TimelineView>>> = [];
  api.listTimeline = vi.fn(() => {
    const deferred = createDeferred<TimelineView>();
    listTimelineDeferreds.push(deferred);
    return deferred.promise;
  });

  const user = userEvent.setup();
  render(<App api={api} />);

  const publishDialog = await openPublishDialog(user);
  await user.type(within(publishDialog).getByPlaceholderText('Write a post'), 'publish without wait');
  await user.click(within(publishDialog).getByRole('button', { name: 'Post' }));

  await waitFor(() => {
    expect(screen.queryByRole('dialog', { name: 'Post' })).not.toBeInTheDocument();
  });
});

test('publish refreshes the active timeline without reloading full shell data', async () => {
  const user = userEvent.setup();
  const api = createDesktopMockApi();
  const originalListDirectMessages = api.listDirectMessages;
  const listDirectMessagesSpy = vi.fn(() => originalListDirectMessages());
  api.listDirectMessages = listDirectMessagesSpy;

  render(<App api={api} />);

  await waitFor(() => {
    expect(listDirectMessagesSpy).toHaveBeenCalledTimes(0);
  });

  const publishDialog = await openPublishDialog(user);
  await user.type(within(publishDialog).getByPlaceholderText('Write a post'), 'local refresh post');
  await user.click(within(publishDialog).getByRole('button', { name: 'Post' }));

  await waitFor(() => {
    expect(screen.getByText('local refresh post')).toBeInTheDocument();
  });
  expect(listDirectMessagesSpy).toHaveBeenCalledTimes(0);
});

