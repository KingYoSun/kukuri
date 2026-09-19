import { describe, expect, test } from 'vitest';

import {
  desktopDistributionFromEnvironment,
  usesSelfManagedUpdater,
} from './distribution';

describe('desktop distribution', () => {
  test('defaults unknown and missing values to the direct distribution', () => {
    expect(desktopDistributionFromEnvironment()).toBe('direct');
    expect(desktopDistributionFromEnvironment('preview')).toBe('direct');
    expect(usesSelfManagedUpdater('direct')).toBe(true);
  });

  test('makes the Microsoft Store the only update owner for Store builds', () => {
    expect(desktopDistributionFromEnvironment('microsoft-store')).toBe('microsoft-store');
    expect(usesSelfManagedUpdater('microsoft-store')).toBe(false);
  });
});
