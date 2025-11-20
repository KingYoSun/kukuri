import { browser } from '@wdio/globals';
import { mkdirSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
import type { Options } from '@wdio/types';
import { startDriver, stopDriver } from './helpers/tauriDriver.ts';

const __dirname = fileURLToPath(new URL('.', import.meta.url));
const PROJECT_ROOT = resolve(__dirname, '..', '..');
const OUTPUT_DIR = join(PROJECT_ROOT, 'tests', 'e2e', 'output');

process.env.VITE_ENABLE_E2E ??= 'true';
process.env.TAURI_ENV_DEBUG ??= 'true';

function runScript(command: string, args: string[]): void {
  const child = spawnSync(command, args, {
    cwd: PROJECT_ROOT,
    stdio: 'inherit',
    shell: process.platform === 'win32'
  });
  if (child.status !== 0) {
    throw new Error(`Command "${command} ${args.join(' ')}" failed with code ${child.status ?? 'unknown'}`);
  }
}

function binaryName(): string {
  if (process.platform === 'win32') {
    return 'kukuri-tauri.exe';
  }
  if (process.platform === 'darwin') {
    return 'kukuri-tauri.app/Contents/MacOS/kukuri-tauri';
  }
  return 'kukuri-tauri';
}

function sanitizeFileName(title: string): string {
  return title.replace(/[^a-zA-Z0-9]+/g, '-').toLowerCase();
}

function pruneUnsupportedCapabilities(target: unknown): void {
  if (!target || typeof target !== 'object') {
    return;
  }
  const record = target as Record<string, unknown>;
  if ('webSocketUrl' in record) {
    delete record.webSocketUrl;
  }
  if ('unhandledPromptBehavior' in record) {
    delete record.unhandledPromptBehavior;
  }

  if ('alwaysMatch' in record) {
    pruneUnsupportedCapabilities(record.alwaysMatch);
  }
  if ('firstMatch' in record && Array.isArray(record.firstMatch)) {
    for (const entry of record.firstMatch) {
      pruneUnsupportedCapabilities(entry);
    }
  }
  if ('desiredCapabilities' in record) {
    pruneUnsupportedCapabilities(record.desiredCapabilities);
  }
}

export const config: Options.Testrunner = {
  runner: 'local',
  specs: [join(__dirname, 'specs/**/*.spec.ts')],
  maxInstances: 1,
  logLevel: 'info',
  waitforTimeout: 15000,
  connectionRetryTimeout: 120000,
  connectionRetryCount: 2,
  autoCompileOpts: {
    autoCompile: true,
    tsNodeOpts: {
      project: './tests/e2e/tsconfig.json',
      transpileOnly: true,
      require: ['tsconfig-paths/register']
    }
  },
  reporters: [
    'spec',
    [
      '@wdio/spec-reporter',
      {
        showPreface: false
      }
    ]
  ],
  framework: 'mocha',
  mochaOpts: {
    ui: 'bdd',
    timeout: 60000
  },
  hostname: '127.0.0.1',
  port: 4445,
  capabilities: [
    {
      browserName: 'wry',
      'wdio:enforceWebDriverClassic': true,
      'tauri:options': {
        application:
          process.env.TAURI_E2E_APP_PATH ??
          join(PROJECT_ROOT, 'src-tauri', 'target', 'debug', binaryName())
      }
    }
  ],
  onPrepare: (_config, capabilities) => {
    mkdirSync(OUTPUT_DIR, { recursive: true });
    if (Array.isArray(capabilities)) {
      for (const capability of capabilities) {
        pruneUnsupportedCapabilities(capability as Record<string, unknown>);
      }
    }
    if (process.env.E2E_SKIP_BUILD === '1') {
      return;
    }
    runScript('pnpm', ['e2e:build']);
  },
  beforeSession: async function (_config, capabilities) {
    pruneUnsupportedCapabilities(capabilities);
    await startDriver();
  },
  afterTest: async function (test, _context, { error }) {
    if (!error) {
      return;
    }
    mkdirSync(OUTPUT_DIR, { recursive: true });
    const fileName = `${Date.now()}-${sanitizeFileName(test?.title ?? 'failure')}.png`;
    const filePath = join(OUTPUT_DIR, fileName);
    await browser.saveScreenshot(filePath);
  },
  onComplete: async () => {
    await stopDriver();
  }
};
