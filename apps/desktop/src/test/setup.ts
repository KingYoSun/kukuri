import '@testing-library/jest-dom/vitest';
import { cleanup } from '@testing-library/react';
import { afterEach, beforeEach, vi } from 'vitest';

import i18n, { DESKTOP_LOCALE_STORAGE_KEY } from '@/i18n';

beforeEach(async () => {
  if (typeof window !== 'undefined') {
    window.localStorage.clear();
    window.localStorage.setItem(DESKTOP_LOCALE_STORAGE_KEY, 'en');
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
