import { renderHook } from '@testing-library/react';
import { afterEach, describe, expect, test, vi } from 'vitest';

import { useFocusScroll } from '@/shell/page/useFocusScroll';

afterEach(() => {
  document.body.replaceChildren();
  vi.restoreAllMocks();
});

describe('useFocusScroll', () => {
  test('retries after readiness changes, then focuses a key only once', () => {
    vi.spyOn(window, 'requestAnimationFrame').mockImplementation((callback) => {
      callback(0);
      return 1;
    });
    vi.spyOn(window, 'cancelAnimationFrame').mockImplementation(() => {});
    const scrollIntoView = vi.fn();
    const focus = vi.fn();

    const hook = renderHook(
      ({ focusKey, readinessKey }) =>
        useFocusScroll({
          focusKey,
          readinessKey,
          selector: '[data-focus-target="target"]',
        }),
      { initialProps: { focusKey: 'thread:post', readinessKey: 0 } }
    );
    expect(scrollIntoView).not.toHaveBeenCalled();

    const target = document.createElement('button');
    target.dataset.focusTarget = 'target';
    target.scrollIntoView = scrollIntoView;
    target.focus = focus;
    document.body.append(target);
    hook.rerender({ focusKey: 'thread:post', readinessKey: 1 });

    expect(scrollIntoView).toHaveBeenCalledWith({ block: 'center' });
    expect(focus).toHaveBeenCalledWith({ preventScroll: true });
    hook.rerender({ focusKey: 'thread:post', readinessKey: 2 });
    expect(scrollIntoView).toHaveBeenCalledTimes(1);

    hook.rerender({ focusKey: 'thread:other-post', readinessKey: 2 });
    expect(scrollIntoView).toHaveBeenCalledTimes(2);
  });

  test('cancels a pending frame on cleanup', () => {
    vi.spyOn(window, 'requestAnimationFrame').mockReturnValue(42);
    const cancelAnimationFrame = vi
      .spyOn(window, 'cancelAnimationFrame')
      .mockImplementation(() => {});

    const hook = renderHook(() =>
      useFocusScroll({
        focusKey: 'live-session',
        readinessKey: 1,
        selector: '[data-live-session-id="live-session"]',
      })
    );
    hook.unmount();
    expect(cancelAnimationFrame).toHaveBeenCalledWith(42);
  });
});
