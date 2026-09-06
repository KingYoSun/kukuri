import { act, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';

import type { NotificationView } from '@/lib/api';
import { createDesktopMockApi } from '@/mocks/desktopApiMock';
import {
  buildNotification,
  buildPaginatedPost,
  getDetailPane,
  paginatePosts,
  renderAtHash,
  setViewportWidth,
} from './DesktopShellPage.testHelpers';

const activation = vi.hoisted(() => ({
  open: undefined as ((notification: NotificationView) => void | Promise<void>) | undefined,
}));
vi.mock('@/shell/useOsNotificationActivation', () => ({
  useOsNotificationActivation: (
    _notifications: NotificationView[],
    onActivate: (notification: NotificationView) => void | Promise<void>
  ) => { activation.open = onActivate; },
}));

const scrollTo = vi.fn();
const originalScrollTo = Object.getOwnPropertyDescriptor(HTMLElement.prototype, 'scrollTo');
beforeEach(() => {
  setViewportWidth(1280);
  window.history.replaceState(null, '', '/');
  activation.open = undefined;
  scrollTo.mockClear();
  vi.spyOn(HTMLElement.prototype, 'clientHeight', 'get').mockReturnValue(300);
  vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockImplementation(function (this: HTMLElement) {
    return {
      top: this.matches('[data-post-object-id]') ? 600 : 100,
      left: 0, right: 400, bottom: 720, width: 400, height: 120,
      x: 0, y: 0, toJSON: () => ({}),
    };
  });
  Object.defineProperty(HTMLElement.prototype, 'scrollTo', {
    configurable: true, value: scrollTo,
  });
});
afterEach(() => {
  vi.restoreAllMocks();
  if (originalScrollTo) Object.defineProperty(HTMLElement.prototype, 'scrollTo', originalScrollTo);
  else Reflect.deleteProperty(HTMLElement.prototype, 'scrollTo');
});

function fixture(objectId: string) {
  const target = buildNotification({
    notification_id: `notification-${objectId}`,
    preview_text: 'Open the exact reply',
    object_id: objectId,
    thread_root_object_id: 'focus-root',
  });
  const posts = Array.from({ length: 45 }, (_, index) =>
    buildPaginatedPost(index + 1, {
      object_id: index === 0 ? 'focus-root' : `focus-reply-${index + 1}`,
      root_id: 'focus-root',
      reply_to: index === 0 ? null : 'focus-root',
      content: `Notification thread post ${index + 1}`,
    })
  );
  const api = createDesktopMockApi({
    notifications: [target],
    seedPosts: {
      'kukuri:topic:general': posts,
    },
  });
  api.listThread = async (topic, thread, cursor, limit = 30) =>
    topic === 'kukuri:topic:general' && thread === 'focus-root'
      ? paginatePosts([...posts].reverse(), cursor ?? null, limit ?? 30)
      : { items: [], next_cursor: null };
  return { target, api };
}

test.each([
  ['os', 'focus-reply-40'], ['in-app', 'focus-reply-40'],
  ['os', 'focus-reply-2'], ['in-app', 'focus-reply-2'],
])('%s notification focuses its exact reply, including older pages: %s', async (origin, objectId) => {
  const { target, api } = fixture(objectId);
  const listThread = vi.spyOn(api, 'listThread');
  renderAtHash(origin === 'os'
    ? '#/timeline?topic=kukuri%3Atopic%3Ageneral'
    : '#/notifications?topic=kukuri%3Atopic%3Ageneral', api);
  if (origin === 'os') {
    await waitFor(() => expect(activation.open).toBeDefined());
    await act(async () => { await activation.open?.(target); });
  } else {
    await userEvent.setup().click(await screen.findByText('Open the exact reply'));
  }
  await waitFor(() => {
    const post = getDetailPane('Thread').querySelector(`[data-post-object-id="${objectId}"]`);
    expect(post).toHaveClass('post-card-targeted');
    expect(post).toHaveFocus();
  });
  expect(window.location.hash).toContain(`focusObjectId=${objectId}`);
  expect(scrollTo).toHaveBeenCalled();
  expect(scrollTo.mock.contexts.every((element) =>
    element === getDetailPane('Thread').querySelector('.shell-column-body'))).toBe(true);
  if (objectId === 'focus-reply-2') {
    expect(listThread.mock.calls.some(([, , cursor]) => cursor !== null)).toBe(true);
  }
  expect(listThread.mock.calls.every(([topic, thread]) =>
    topic === 'kukuri:topic:general' && thread === 'focus-root')).toBe(true);
});

test('a repeated OS click scrolls again but an older-page refresh does not', async () => {
  const { target, api } = fixture('focus-reply-40');
  renderAtHash('#/timeline?topic=kukuri%3Atopic%3Ageneral', api);
  await waitFor(() => expect(activation.open).toBeDefined());
  await act(async () => { await activation.open?.(target); });
  await waitFor(() => expect(scrollTo).toHaveBeenCalledTimes(1));
  const user = userEvent.setup();
  await user.click(within(getDetailPane('Thread')).getByRole('button', { name: 'Load more' }));
  expect(scrollTo).toHaveBeenCalledTimes(1);
  await act(async () => { await activation.open?.(target); });
  await waitFor(() => expect(scrollTo).toHaveBeenCalledTimes(2));
}, 15_000); // Multiple 45-row renders plus pagination; this is not a performance budget.

test('a missing notification target opens its thread without focusing a different post', async () => {
  const { target, api } = fixture('missing-reply');
  renderAtHash('#/timeline?topic=kukuri%3Atopic%3Ageneral', api);
  await waitFor(() => expect(activation.open).toBeDefined());
  await act(async () => { await activation.open?.(target); });
  await waitFor(() => expect(getDetailPane('Thread')).toBeInTheDocument());
  expect(getDetailPane('Thread').querySelector('.post-card-targeted')).toBeNull();
  expect(scrollTo).not.toHaveBeenCalled();
  expect(window.location.hash).not.toContain('focusObjectId');
});

test.each(['os', 'in-app'])('%s notification keeps its private channel while focusing the post', async (origin) => {
  const api = createDesktopMockApi();
  const channel = await api.createPrivateChannel('kukuri:topic:general', 'private focus', 'invite_only');
  const target = buildNotification({
    object_id: 'private-target', thread_root_object_id: 'private-root',
    channel_id: channel.channel_id, preview_text: 'Private notification target',
  });
  api.listNotifications = async () => [target];
  api.listThread = vi.fn(async () => ({
    items: [buildPaginatedPost(1, {
      object_id: 'private-target', root_id: 'private-root', channel_id: channel.channel_id,
      content: 'Private target content', audience_label: 'Private',
    })], next_cursor: null,
  }));
  renderAtHash('#/timeline?topic=kukuri%3Atopic%3Ageneral', api);
  await screen.findByText('Private notification target');
  if (origin === 'os') await act(async () => { await activation.open?.(target); });
  else await userEvent.setup().click(screen.getByText('Private notification target'));
  await waitFor(() => {
    expect(window.location.hash).toContain(`channel=${channel.channel_id}`);
    expect(getDetailPane('Thread').querySelector('[data-post-object-id="private-target"]')).toHaveFocus();
  });
  expect(api.listThread).toHaveBeenCalledWith('kukuri:topic:general', 'private-root', null, 30);
});
