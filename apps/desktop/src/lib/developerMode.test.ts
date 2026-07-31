import { describe, expect, it } from 'vitest';

import {
  DEVELOPER_MODE_STORAGE_KEY,
  readDeveloperMode,
  writeDeveloperMode,
} from '@/lib/developerMode';

describe('developerMode', () => {
  it('defaults to disabled when nothing is stored', () => {
    window.localStorage.removeItem(DEVELOPER_MODE_STORAGE_KEY);

    expect(readDeveloperMode()).toBe(false);
  });

  it('reads enabled state only for the literal true value', () => {
    window.localStorage.setItem(DEVELOPER_MODE_STORAGE_KEY, 'true');
    expect(readDeveloperMode()).toBe(true);

    window.localStorage.setItem(DEVELOPER_MODE_STORAGE_KEY, '1');
    expect(readDeveloperMode()).toBe(false);

    window.localStorage.setItem(DEVELOPER_MODE_STORAGE_KEY, 'false');
    expect(readDeveloperMode()).toBe(false);
  });

  it('persists writes across reads', () => {
    writeDeveloperMode(true);
    expect(window.localStorage.getItem(DEVELOPER_MODE_STORAGE_KEY)).toBe('true');
    expect(readDeveloperMode()).toBe(true);

    writeDeveloperMode(false);
    expect(window.localStorage.getItem(DEVELOPER_MODE_STORAGE_KEY)).toBe('false');
    expect(readDeveloperMode()).toBe(false);
  });
});
