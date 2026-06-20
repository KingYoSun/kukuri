import { renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';

import type { NotificationView } from '@/lib/api';

const listenMock = vi.fn();

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
    listenMock.mockReset();
    listenMock.mockResolvedValue(() => undefined);
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {};
  });

  afterEach(() => {
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
  });

  test('does not subscribe outside the Tauri runtime', () => {
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
    renderHook(() => useOsNotificationActivation([notification()], vi.fn()));
    expect(listenMock).not.toHaveBeenCalled();
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
});
