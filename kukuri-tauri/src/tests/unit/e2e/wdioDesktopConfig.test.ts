import { describe, expect, it, vi } from 'vitest';

const E2E_FORBID_PENDING = 'E2E_FORBID_PENDING';
const E2E_MOCHA_TIMEOUT_MS = 'E2E_MOCHA_TIMEOUT_MS';
const PATH_KEY = 'PATH';

const importConfig = async (options?: {
  forbidPending?: string;
  mochaTimeoutMs?: string;
  path?: string;
}) => {
  const previous = process.env[E2E_FORBID_PENDING];
  const previousTimeout = process.env[E2E_MOCHA_TIMEOUT_MS];
  const previousPath = process.env[PATH_KEY];

  if (options?.forbidPending === undefined) {
    delete process.env[E2E_FORBID_PENDING];
  } else {
    process.env[E2E_FORBID_PENDING] = options.forbidPending;
  }

  if (options?.mochaTimeoutMs === undefined) {
    delete process.env[E2E_MOCHA_TIMEOUT_MS];
  } else {
    process.env[E2E_MOCHA_TIMEOUT_MS] = options.mochaTimeoutMs;
  }

  if (options?.path === undefined) {
    delete process.env[PATH_KEY];
  } else {
    process.env[PATH_KEY] = options.path;
  }

  try {
    vi.resetModules();
    const module = await import('../../../../tests/e2e/wdio.desktop.ts');
    return {
      config: module.config,
      pathAfterImport: process.env[PATH_KEY],
    };
  } finally {
    if (previous === undefined) {
      delete process.env[E2E_FORBID_PENDING];
    } else {
      process.env[E2E_FORBID_PENDING] = previous;
    }

    if (previousTimeout === undefined) {
      delete process.env[E2E_MOCHA_TIMEOUT_MS];
    } else {
      process.env[E2E_MOCHA_TIMEOUT_MS] = previousTimeout;
    }

    if (previousPath === undefined) {
      delete process.env[PATH_KEY];
    } else {
      process.env[PATH_KEY] = previousPath;
    }
  }
};

describe('wdio.desktop pending enforcement', () => {
  it('keeps forbidPending disabled by default', async () => {
    const { config } = await importConfig();
    expect(config.mochaOpts?.forbidPending).toBe(false);
  });

  it('enables forbidPending when E2E_FORBID_PENDING=1', async () => {
    const { config } = await importConfig({ forbidPending: '1' });
    expect(config.mochaOpts?.forbidPending).toBe(true);
  });

  it('uses E2E_MOCHA_TIMEOUT_MS when provided', async () => {
    const { config } = await importConfig({ mochaTimeoutMs: '180000' });
    expect(config.mochaOpts?.timeout).toBe(180000);
  });

  it('prepends cargo bin path when PATH does not include it', async () => {
    const { pathAfterImport } = await importConfig({ path: '/usr/bin:/bin' });
    expect(pathAfterImport?.startsWith('/usr/local/cargo/bin:')).toBe(true);
  });
});
