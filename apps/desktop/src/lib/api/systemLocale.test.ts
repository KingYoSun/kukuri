import { beforeEach, expect, test, vi } from 'vitest';
const mocks = vi.hoisted(() => ({ invoke:vi.fn(), isTauri:vi.fn() }));
vi.mock('@tauri-apps/api/core', () => mocks);
import { getSystemLocales } from './systemLocale';
import { createDesktopMockApi } from '@/mocks/desktopApiMock';

beforeEach(() => {
  mocks.invoke.mockReset();
  mocks.isTauri.mockReturnValue(false);
  delete window.__KUKURI_DESKTOP__;
});

test('browser and Storybook never invoke native locale commands', async () => {
  expect(await getSystemLocales()).toEqual([]);
  mocks.isTauri.mockReturnValue(true);
  window.__KUKURI_DESKTOP__ = createDesktopMockApi();
  expect(await getSystemLocales()).toEqual([]);
  expect(mocks.invoke).not.toHaveBeenCalled();
});

test('native startup only requests the argument-free local locale command', async () => {
  mocks.isTauri.mockReturnValue(true);
  mocks.invoke.mockResolvedValue(['ja-JP', 'en-US']);
  expect(await getSystemLocales()).toEqual(['ja-JP','en-US']);
  expect(mocks.invoke.mock.calls).toEqual([['get_system_locales', undefined]]);
});
