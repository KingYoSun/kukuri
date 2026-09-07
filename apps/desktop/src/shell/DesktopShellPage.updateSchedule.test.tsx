import { act, cleanup } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';

const { updaterCheck, invoke } = vi.hoisted(() => ({
  updaterCheck: vi.fn(),
  invoke: vi.fn(async () => undefined),
}));
vi.mock('@/lib/appUpdater', () => ({ check: updaterCheck }));
vi.mock('@tauri-apps/api/app', () => ({ getVersion: async () => '0.1.8' }));
vi.mock('@tauri-apps/api/core', () => ({ invoke }));
vi.mock('@tauri-apps/api/event', () => ({ listen: async () => () => {} }));
// OS activation is unrelated to the actual shell update scheduler tested here.
vi.mock('@/shell/useOsNotificationActivation', () => ({ useOsNotificationActivation: () => {} }));

import { renderAtHash, setViewportWidth } from './DesktopShellPage.testHelpers';
import { appUpdateStore, INITIAL_UPDATE_STATE } from './useAppUpdateStore';

afterEach(() => {
  cleanup();
  Reflect.deleteProperty(window, '__TAURI_INTERNALS__');
  appUpdateStore.setState({ updateState: { ...INITIAL_UPDATE_STATE }, pendingUpdate: null });
  vi.useRealTimers();
});

test('the real shell scheduler preserves a downloaded update across periodic checks', async () => {
  vi.useFakeTimers({ toFake: ['setInterval', 'clearInterval'] });
  const schedule = globalThis.setInterval;
  // Isolate the update timer from unrelated 3-second timeline polling.
  vi.spyOn(window, 'setInterval').mockImplementation((handler, delay, ...args) => {
    const timer = schedule(handler, delay, ...args);
    if (delay !== 30 * 60 * 1000) window.clearInterval(timer);
    return timer;
  });
  Object.defineProperty(window, '__TAURI_INTERNALS__', { configurable: true, value: {} });
  setViewportWidth(1280);
  const update = {
    version: '0.1.9',
    download: vi.fn(async () => undefined),
    install: vi.fn(async () => undefined),
  };
  updaterCheck.mockReset().mockResolvedValue(update);
  invoke.mockClear();
  appUpdateStore.setState({ updateState: { ...INITIAL_UPDATE_STATE }, pendingUpdate: null });
  const realCheck = appUpdateStore.getState().checkForUpdate;
  const check = vi.fn(realCheck);
  appUpdateStore.setState({ checkForUpdate: check });
  try {
    let view!: ReturnType<typeof renderAtHash>;
    await act(async () => { view = renderAtHash('#/timeline'); });
    expect(check).toHaveBeenCalledTimes(1);
    expect(updaterCheck).toHaveBeenCalledTimes(1);
    await act(async () => { await appUpdateStore.getState().downloadUpdate(); });
    const verified = appUpdateStore.getState().updateState;
    expect(verified.status).toBe('ready_to_restart');

    // Cross two real 30-minute scheduler ticks without waiting in wall-clock time.
    await act(async () => { await vi.advanceTimersByTimeAsync(60 * 60 * 1000); });
    expect(check).toHaveBeenCalledTimes(3);
    expect(updaterCheck).toHaveBeenCalledTimes(1);
    expect(appUpdateStore.getState().pendingUpdate).toBe(update);
    expect(appUpdateStore.getState().updateState).toEqual(verified);
    expect(update.download).toHaveBeenCalledTimes(1);
    expect(update.install).not.toHaveBeenCalled();
    expect(invoke).not.toHaveBeenCalledWith('restart_after_update');

    view.unmount();
    await act(async () => { await vi.advanceTimersByTimeAsync(30 * 60 * 1000); });
    expect(check).toHaveBeenCalledTimes(3);
    await appUpdateStore.getState().restartAndInstall();
    expect(update.install).toHaveBeenCalledTimes(1);
    expect(invoke).toHaveBeenCalledWith('restart_after_update');
  } finally {
    appUpdateStore.setState({ checkForUpdate: realCheck });
  }
}, 15000);
