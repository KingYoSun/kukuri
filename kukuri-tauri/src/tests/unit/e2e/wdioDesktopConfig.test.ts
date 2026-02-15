import { describe, expect, it, vi } from 'vitest';

const E2E_FORBID_PENDING = 'E2E_FORBID_PENDING';

const importConfig = async (forbidPending?: string) => {
  const previous = process.env[E2E_FORBID_PENDING];
  if (forbidPending === undefined) {
    delete process.env[E2E_FORBID_PENDING];
  } else {
    process.env[E2E_FORBID_PENDING] = forbidPending;
  }

  try {
    vi.resetModules();
    const module = await import('../../../../tests/e2e/wdio.desktop.ts');
    return module.config;
  } finally {
    if (previous === undefined) {
      delete process.env[E2E_FORBID_PENDING];
    } else {
      process.env[E2E_FORBID_PENDING] = previous;
    }
  }
};

describe('wdio.desktop pending enforcement', () => {
  it('keeps forbidPending disabled by default', async () => {
    const config = await importConfig();
    expect(config.mochaOpts?.forbidPending).toBe(false);
  });

  it('enables forbidPending when E2E_FORBID_PENDING=1', async () => {
    const config = await importConfig('1');
    expect(config.mochaOpts?.forbidPending).toBe(true);
  });
});
