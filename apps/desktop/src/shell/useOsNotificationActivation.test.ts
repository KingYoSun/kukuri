import { renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';

import type { NotificationView } from '@/lib/api';

const listenMock = vi.fn();
const deepLinkMock = vi.hoisted(() => ({ onOpenUrl: vi.fn(), getCurrent: vi.fn() }));

vi.mock('@tauri-apps/plugin-deep-link', () => deepLinkMock);

vi.mock('@tauri-apps/api/event', () => ({
  listen: (...args: unknown[]) => listenMock(...args),
}));

import { useOsNotificationActivation } from './useOsNotificationActivation';

type EventCallback = (event: { payload: { notification_id: string } }) => void;

function notification(overrides: Partial<NotificationView> = {}): NotificationView {
  return {
    notification_id: 'notif-1',
    kind: 'reply',
    actor_pubkey: 'actor-pubkey',
    created_at: 0,
    received_at: 0,
    ...overrides,
  } as NotificationView;
}

describe('useOsNotificationActivation', () => {
  beforeEach(() => {
    sessionStorage.clear();
    listenMock.mockReset();
    listenMock.mockResolvedValue(() => undefined);
    deepLinkMock.onOpenUrl.mockReset().mockResolvedValue(() => undefined);
    deepLinkMock.getCurrent.mockReset().mockResolvedValue(null);
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {};
  });

  afterEach(() => {
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
  });

  test('does not subscribe outside the Tauri runtime', () => {
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
    renderHook(() => useOsNotificationActivation([notification()], vi.fn()));
    expect(listenMock).not.toHaveBeenCalled();
    expect(deepLinkMock.onOpenUrl).not.toHaveBeenCalled();
  });

  test.each(['kukuri://notification?id=', 'kukuri://notification/?id='])(
    'opens the matching notification from a Windows protocol activation: %s',
    async (prefix) => {
      let onUrl: ((urls: string[]) => void) | undefined;
      deepLinkMock.onOpenUrl.mockImplementation(async (callback) => {
        onUrl = callback;
        return () => undefined;
      });
      const target = notification({ notification_id: 'notification:recipient:reply:target' });
      const onActivate = vi.fn();
      renderHook(() => useOsNotificationActivation([target], onActivate));
      await vi.waitFor(() => expect(onUrl).toBeDefined());
      onUrl?.([`${prefix}${encodeURIComponent(target.notification_id)}`]);
      expect(onActivate).toHaveBeenCalledExactlyOnceWith(target);
    }
  );

  test.each(['kukuri://notification?id=', 'kukuri://notification/?id='])(
    'resolves a notification URI received before the frontend mounted: %s',
    async (prefix) => {
      const target = notification();
      deepLinkMock.getCurrent.mockResolvedValue([`${prefix}notif-1`]);
      const onActivate = vi.fn();
      renderHook(() => useOsNotificationActivation([target], onActivate));
      await vi.waitFor(() => expect(onActivate).toHaveBeenCalledExactlyOnceWith(target));
    }
  );

  test('a live notification click supersedes an older initial URI', async () => {
    let onUrl: ((urls: string[]) => void) | undefined;
    let resolveInitial: ((urls: string[]) => void) | undefined;
    deepLinkMock.getCurrent.mockReturnValue(new Promise<string[]>((resolve) => {
      resolveInitial = resolve;
    }));
    deepLinkMock.onOpenUrl.mockImplementation(async (callback) => {
      onUrl = callback;
      return () => undefined;
    });
    const older = notification();
    const latest = notification({ notification_id: 'latest' });
    const onActivate = vi.fn();
    renderHook(() => useOsNotificationActivation([older, latest], onActivate));
    await vi.waitFor(() => expect(deepLinkMock.getCurrent).toHaveBeenCalled());
    onUrl?.(['kukuri://notification?id=latest']);
    resolveInitial?.(['kukuri://notification?id=notif-1']);
    await Promise.resolve();
    expect(onActivate).toHaveBeenCalledExactlyOnceWith(latest);
  });

  test('waits for the current notification list after protocol activation', async () => {
    let onUrl: ((urls: string[]) => void) | undefined;
    deepLinkMock.onOpenUrl.mockImplementation(async (callback) => {
      onUrl = callback;
      return () => undefined;
    });
    const target = notification();
    const onActivate = vi.fn();
    const { rerender } = renderHook(
      ({ items }) => useOsNotificationActivation(items, onActivate),
      { initialProps: { items: [] as NotificationView[] } }
    );
    await vi.waitFor(() => expect(onUrl).toBeDefined());
    onUrl?.(['kukuri://notification?id=notif-1']);
    expect(onActivate).not.toHaveBeenCalled();
    rerender({ items: [target] });
    expect(onActivate).toHaveBeenCalledExactlyOnceWith(target);
    rerender({ items: [{ ...target }] });
    expect(onActivate).toHaveBeenCalledTimes(1);
  });

  test('does not replay the initial URI on remount but allows another live click', async () => {
    const callbacks: Array<(urls: string[]) => void> = [];
    const disposeUrl = vi.fn();
    deepLinkMock.onOpenUrl.mockImplementation(async (callback) => {
      callbacks.push(callback);
      return disposeUrl;
    });
    deepLinkMock.getCurrent.mockResolvedValue(['kukuri://notification?id=notif-1']);
    const target = notification();
    const onActivate = vi.fn();
    const first = renderHook(() => useOsNotificationActivation([target], onActivate));
    await vi.waitFor(() => expect(onActivate).toHaveBeenCalledTimes(1));
    first.unmount();
    expect(disposeUrl).toHaveBeenCalledTimes(1);
    renderHook(() => useOsNotificationActivation([target], onActivate));
    await vi.waitFor(() => expect(deepLinkMock.getCurrent).toHaveBeenCalledTimes(2));
    await Promise.resolve();
    expect(onActivate).toHaveBeenCalledTimes(1);
    callbacks[0](['kukuri://notification?id=notif-1']);
    expect(onActivate).toHaveBeenCalledTimes(1);
    callbacks[1](['kukuri://notification?id=notif-1']);
    expect(onActivate).toHaveBeenCalledTimes(2);
  });

  test('does not open an unknown or another account notification from a URI', async () => {
    let onUrl: ((urls: string[]) => void) | undefined;
    deepLinkMock.onOpenUrl.mockImplementation(async (callback) => {
      onUrl = callback;
      return () => undefined;
    });
    const target = notification();
    const onActivate = vi.fn();
    renderHook(() => useOsNotificationActivation([target], onActivate));
    await vi.waitFor(() => expect(onUrl).toBeDefined());
    for (const url of [
      'kukuri://notification?id=another-account-id',
      'kukuri://notification?id=notif-1&profile=other',
      'kukuri://notification?id=notif-1#extra',
      'kukuri://notification?id=%00',
      'https://example.com/?id=notif-1',
    ]) onUrl?.([url]);
    expect(onActivate).not.toHaveBeenCalled();
  });

  test('disposes a URL listener that finishes registering after unmount', async () => {
    let finishRegistration: ((dispose: () => void) => void) | undefined;
    const dispose = vi.fn();
    deepLinkMock.onOpenUrl.mockReturnValue(new Promise<() => void>((resolve) => {
      finishRegistration = resolve;
    }));
    const { unmount } = renderHook(() => useOsNotificationActivation([], vi.fn()));
    await vi.waitFor(() => expect(deepLinkMock.onOpenUrl).toHaveBeenCalled());
    unmount();
    finishRegistration?.(dispose);
    await vi.waitFor(() => expect(dispose).toHaveBeenCalledTimes(1));
    expect(deepLinkMock.getCurrent).not.toHaveBeenCalled();
  });

  test('opens the matching notification when activated', async () => {
    let capturedCallback: EventCallback | undefined;
    listenMock.mockImplementation(async (_event: string, cb: EventCallback) => {
      capturedCallback = cb;
      return () => undefined;
    });
    const onActivate = vi.fn();
    const target = notification({ notification_id: 'notif-2' });

    renderHook(() =>
      useOsNotificationActivation([notification(), target], onActivate)
    );

    // Wait for the async listen() registration to resolve.
    await vi.waitFor(() => expect(capturedCallback).toBeDefined());

    capturedCallback?.({ payload: { notification_id: 'notif-2' } });
    expect(onActivate).toHaveBeenCalledWith(target);
  });

  test('ignores activation for an unknown notification id', async () => {
    let capturedCallback: EventCallback | undefined;
    listenMock.mockImplementation(async (_event: string, cb: EventCallback) => {
      capturedCallback = cb;
      return () => undefined;
    });
    const onActivate = vi.fn();

    renderHook(() => useOsNotificationActivation([notification()], onActivate));
    await vi.waitFor(() => expect(capturedCallback).toBeDefined());

    capturedCallback?.({ payload: { notification_id: 'missing' } });
    expect(onActivate).not.toHaveBeenCalled();
  });

  test('opens a clicked notification once after the hidden-window list refreshes', async () => {
    let capturedCallback: EventCallback | undefined;
    listenMock.mockImplementation(async (_event: string, cb: EventCallback) => {
      capturedCallback = cb;
      return () => undefined;
    });
    const onActivate = vi.fn();
    const target = notification({ notification_id: 'received-while-hidden' });
    const { rerender } = renderHook(
      ({ items }) => useOsNotificationActivation(items, onActivate),
      { initialProps: { items: [] as NotificationView[] } }
    );
    await vi.waitFor(() => expect(capturedCallback).toBeDefined());

    capturedCallback?.({ payload: { notification_id: target.notification_id } });
    expect(onActivate).not.toHaveBeenCalled();
    rerender({ items: [target] });
    expect(onActivate).toHaveBeenCalledExactlyOnceWith(target);
    rerender({ items: [{ ...target, read_at: 1 }] });
    expect(onActivate).toHaveBeenCalledTimes(1);
  });

  test('only the latest pending click opens, using the latest navigation callback', async () => {
    let capturedCallback: EventCallback | undefined;
    listenMock.mockImplementation(async (_event: string, cb: EventCallback) => {
      capturedCallback = cb;
      return () => undefined;
    });
    const previousActivate = vi.fn();
    const latestActivate = vi.fn();
    const older = notification({ notification_id: 'older' });
    const latest = notification({ notification_id: 'latest' });
    const { rerender } = renderHook(
      ({ items, onActivate }) => useOsNotificationActivation(items, onActivate),
      { initialProps: { items: [] as NotificationView[], onActivate: previousActivate } }
    );
    await vi.waitFor(() => expect(capturedCallback).toBeDefined());

    capturedCallback?.({ payload: { notification_id: older.notification_id } });
    capturedCallback?.({ payload: { notification_id: latest.notification_id } });
    rerender({ items: [older], onActivate: latestActivate });
    expect(previousActivate).not.toHaveBeenCalled();
    expect(latestActivate).not.toHaveBeenCalled();
    rerender({ items: [older, latest], onActivate: latestActivate });
    expect(previousActivate).not.toHaveBeenCalled();
    expect(latestActivate).toHaveBeenCalledExactlyOnceWith(latest);
    expect(listenMock).toHaveBeenCalledTimes(1);
  });

  test('does not navigate from a disposed listener or carry a pending id into a new mount', async () => {
    const callbacks: EventCallback[] = [];
    const dispose = vi.fn();
    listenMock.mockImplementation(async (_event: string, cb: EventCallback) => {
      callbacks.push(cb);
      return dispose;
    });
    const target = notification();
    const onActivate = vi.fn();
    const first = renderHook(() => useOsNotificationActivation([], onActivate));
    await vi.waitFor(() => expect(callbacks).toHaveLength(1));
    callbacks[0]({ payload: { notification_id: target.notification_id } });
    first.unmount();
    expect(dispose).toHaveBeenCalledTimes(1);

    renderHook(() => useOsNotificationActivation([target], onActivate));
    await vi.waitFor(() => expect(callbacks).toHaveLength(2));
    callbacks[0]({ payload: { notification_id: target.notification_id } });
    expect(onActivate).not.toHaveBeenCalled();
    callbacks[1]({ payload: { notification_id: target.notification_id } });
    expect(onActivate).toHaveBeenCalledExactlyOnceWith(target);
  });
});
