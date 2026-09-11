import { render } from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn(async (): Promise<void> => undefined) }));
vi.mock('@tauri-apps/api/core', () => ({ invoke }));

import { useDeveloperModeBridge } from './useDeveloperModeBridge';

function Bridge({ enabled }: { enabled: boolean }) {
  useDeveloperModeBridge(enabled);
  return null;
}

beforeEach(() => {
  invoke.mockClear();
});

afterEach(() => {
  delete (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
});

// #978: backend は開発者モードを永続化しないため、mount 時と変更時に必ず送る。
test('mirrors the developer mode into the backend on mount and on every change', () => {
  (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ = {};
  const { rerender } = render(<Bridge enabled={false} />);
  expect(invoke).toHaveBeenCalledTimes(1);
  expect(invoke).toHaveBeenLastCalledWith('set_developer_mode_enabled', { enabled: false });

  rerender(<Bridge enabled />);
  expect(invoke).toHaveBeenCalledTimes(2);
  expect(invoke).toHaveBeenLastCalledWith('set_developer_mode_enabled', { enabled: true });

  rerender(<Bridge enabled />);
  expect(invoke).toHaveBeenCalledTimes(2);
});

test('does nothing outside the Tauri runtime and survives a rejected mirror', async () => {
  render(<Bridge enabled />);
  expect(invoke).not.toHaveBeenCalled();

  (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ = {};
  invoke.mockRejectedValueOnce(new Error('desktop command requires Ready startup state'));
  render(<Bridge enabled />);
  expect(invoke).toHaveBeenCalledTimes(1);
  await Promise.resolve();
});
