import '@testing-library/jest-dom/vitest';
import { cleanup } from '@testing-library/react';
import { afterEach, beforeEach, vi } from 'vitest';

import i18n, { DESKTOP_LOCALE_STORAGE_KEY } from '@/i18n';
import { DEVELOPER_MODE_STORAGE_KEY } from '@/lib/developerMode';

beforeEach(async () => {
  if (typeof window !== 'undefined') {
    window.localStorage.clear();
    window.localStorage.setItem(DESKTOP_LOCALE_STORAGE_KEY, 'en');
    // 既存テストは Live/Game タブや診断表示を前提とするため developer mode を既定で有効化する。
    // 既定 OFF の挙動は DesktopShellPage.developerMode.test.tsx が明示的に上書きして検証する。
    window.localStorage.setItem(DEVELOPER_MODE_STORAGE_KEY, 'true');
  }
  await i18n.changeLanguage('en');
  if (typeof document !== 'undefined') {
    document.documentElement.lang = 'en';
  }
});

afterEach(() => {
  vi.restoreAllMocks();
  vi.useRealTimers();
  if (typeof window !== 'undefined') {
    window.localStorage.clear();
  }
  if (typeof document !== 'undefined') {
    delete document.documentElement.dataset.theme;
    document.documentElement.lang = 'en';
  }
  cleanup();
});
