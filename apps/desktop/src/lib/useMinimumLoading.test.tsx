// #994: 一覧の初回 loading は表示開始から最低 MIN_LIST_LOADING_MS 続け、取得が長い場合は完了まで続ける。
// 表示用 status だけを遅らせ、取得の開始・回数は変えない(INVAR-4)。
import { act, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';

import type { ExtendedPanelStatus } from '@/components/extended/types';

import { MIN_LIST_LOADING_MS, useMinimumLoading } from './useMinimumLoading';

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  vi.useRealTimers();
});

test('holds loading until the minimum duration when the fetch finishes early', () => {
  const { result, rerender } = renderHook(({ status }: { status: ExtendedPanelStatus }) => useMinimumLoading(status), {
    initialProps: { status: 'loading' as ExtendedPanelStatus },
  });
  expect(result.current).toBe('loading');

  act(() => {
    vi.advanceTimersByTime(200);
  });
  rerender({ status: 'ready' });
  expect(result.current).toBe('loading');

  act(() => {
    vi.advanceTimersByTime(MIN_LIST_LOADING_MS - 200 - 1);
  });
  expect(result.current).toBe('loading');

  act(() => {
    vi.advanceTimersByTime(1);
  });
  expect(result.current).toBe('ready');
});

test('shows the result immediately when the fetch already took longer than the minimum', () => {
  const { result, rerender } = renderHook(({ status }: { status: ExtendedPanelStatus }) => useMinimumLoading(status), {
    initialProps: { status: 'loading' as ExtendedPanelStatus },
  });
  act(() => {
    vi.advanceTimersByTime(MIN_LIST_LOADING_MS + 700);
  });
  rerender({ status: 'ready' });
  expect(result.current).toBe('ready');
});

test('error results are held for the same minimum so the notice does not flash', () => {
  const { result, rerender } = renderHook(({ status }: { status: ExtendedPanelStatus }) => useMinimumLoading(status), {
    initialProps: { status: 'loading' as ExtendedPanelStatus },
  });
  rerender({ status: 'error' });
  expect(result.current).toBe('loading');
  act(() => {
    vi.advanceTimersByTime(MIN_LIST_LOADING_MS);
  });
  expect(result.current).toBe('error');
});

test('a panel that mounts with data ready never shows loading', () => {
  const { result } = renderHook(() => useMinimumLoading('ready'));
  expect(result.current).toBe('ready');
});

test('a refresh that re-enters loading is held again and applies the result afterwards', () => {
  const { result, rerender } = renderHook(({ status }: { status: ExtendedPanelStatus }) => useMinimumLoading(status), {
    initialProps: { status: 'ready' as ExtendedPanelStatus },
  });
  rerender({ status: 'loading' });
  expect(result.current).toBe('loading');
  rerender({ status: 'ready' });
  expect(result.current).toBe('loading');
  act(() => {
    vi.advanceTimersByTime(MIN_LIST_LOADING_MS);
  });
  expect(result.current).toBe('ready');
});

test('unmount clears the pending timer', () => {
  const clearSpy = vi.spyOn(globalThis, 'clearTimeout');
  const { rerender, unmount } = renderHook(({ status }: { status: ExtendedPanelStatus }) => useMinimumLoading(status), {
    initialProps: { status: 'loading' as ExtendedPanelStatus },
  });
  rerender({ status: 'ready' });
  const callsBefore = clearSpy.mock.calls.length;
  unmount();
  expect(clearSpy.mock.calls.length).toBeGreaterThan(callsBefore);
  expect(vi.getTimerCount()).toBe(0);
});
