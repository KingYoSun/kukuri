import '@testing-library/jest-dom/vitest';
import { cleanup } from '@testing-library/react';
import { afterEach, vi } from 'vitest';

afterEach(() => {
  vi.restoreAllMocks();
  vi.useRealTimers();
  if (typeof window !== 'undefined') {
    window.localStorage.clear();
  }
  if (typeof document !== 'undefined') {
    delete document.documentElement.dataset.theme;
  }
  cleanup();
});
