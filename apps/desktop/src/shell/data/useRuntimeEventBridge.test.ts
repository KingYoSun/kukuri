import { renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';

import type { RuntimeEvent } from '@/lib/api';
import { createDesktopMockApi } from '@/mocks/desktopApiMock';

const listenMock = vi.fn();

vi.mock('@tauri-apps/api/event', () => ({
  listen: (...args: unknown[]) => listenMock(...args),
}));

import { useRuntimeEventBridge } from './useRuntimeEventBridge';

type EventCallback = (event: { payload: RuntimeEvent }) => void;

describe('useRuntimeEventBridge', () => {
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
    renderHook(() => useRuntimeEventBridge(vi.fn(), vi.fn()));
    expect(listenMock).not.toHaveBeenCalled();
  });

  test('dispatches notification and partial sync status events without API calls', async () => {
    let capturedCallback: EventCallback | undefined;
    listenMock.mockImplementation(async (_event: string, callback: EventCallback) => {
      capturedCallback = callback;
      return () => undefined;
    });
    const onNotificationStatusChanged = vi.fn();
    const onSyncStatusChanged = vi.fn();
    const api = createDesktopMockApi();
    const syncStatus = await api.getSyncStatus();
    const communityNodeStatuses = await api.getCommunityNodeStatuses();

    renderHook(() =>
      useRuntimeEventBridge(onNotificationStatusChanged, onSyncStatusChanged)
    );
    await vi.waitFor(() => expect(capturedCallback).toBeDefined());

    capturedCallback?.({ payload: { type: 'notification_status_changed' } });
    expect(onNotificationStatusChanged).toHaveBeenCalledTimes(1);

    capturedCallback?.({
      payload: {
        type: 'sync_status_changed',
        sync_status: syncStatus,
        community_node_statuses: null,
      },
    });
    capturedCallback?.({
      payload: {
        type: 'sync_status_changed',
        sync_status: null,
        community_node_statuses: communityNodeStatuses,
      },
    });
    expect(onSyncStatusChanged).toHaveBeenNthCalledWith(1, syncStatus, null);
    expect(onSyncStatusChanged).toHaveBeenNthCalledWith(2, null, communityNodeStatuses);
  });

  test('ignores unknown runtime event types', async () => {
    let capturedCallback: EventCallback | undefined;
    listenMock.mockImplementation(async (_event: string, callback: EventCallback) => {
      capturedCallback = callback;
      return () => undefined;
    });
    const onNotificationStatusChanged = vi.fn();
    const onSyncStatusChanged = vi.fn();

    renderHook(() =>
      useRuntimeEventBridge(onNotificationStatusChanged, onSyncStatusChanged)
    );
    await vi.waitFor(() => expect(capturedCallback).toBeDefined());

    capturedCallback?.({ payload: { type: 'future_event' } as unknown as RuntimeEvent });
    expect(onNotificationStatusChanged).not.toHaveBeenCalled();
    expect(onSyncStatusChanged).not.toHaveBeenCalled();
  });
});
