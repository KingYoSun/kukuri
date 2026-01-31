import { browser } from '@wdio/globals';
import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
import { createConnection } from 'node:net';
import type { Options } from '@wdio/types';
import { startDriver, stopDriver } from './helpers/tauriDriver.ts';
import { startCommunityNodeMock, stopCommunityNodeMock } from './helpers/communityNodeMock.ts';

const __dirname = fileURLToPath(new URL('.', import.meta.url));
const PROJECT_ROOT = resolve(__dirname, '..', '..');
const OUTPUT_DIR = join(PROJECT_ROOT, 'tests', 'e2e', 'output');
const CLI_BOOTSTRAP_PATH =
  process.env.KUKURI_CLI_BOOTSTRAP_PATH ?? join(OUTPUT_DIR, 'cli_bootstrap_nodes.json');

let shouldStopCommunityNodeMock = false;

process.env.KUKURI_BOOTSTRAP_PEERS = '';
process.env.WDIO_WORKERS ??= '1';
process.env.WDIO_MAX_WORKERS ??= process.env.WDIO_WORKERS;
process.env.TAURI_DRIVER_PORT ??= String(4700 + Math.floor(Math.random() * 400));
process.env.KUKURI_CLI_BOOTSTRAP_PATH = CLI_BOOTSTRAP_PATH;

const WORKER_COUNT = Number(process.env.WDIO_WORKERS ?? process.env.WDIO_MAX_WORKERS ?? '1');
console.info(`[wdio.desktop] worker count resolved to ${WORKER_COUNT}`);
console.info(`[wdio.desktop] driver port resolved to ${process.env.TAURI_DRIVER_PORT}`);
console.info(`[wdio.desktop] cli bootstrap path resolved to ${CLI_BOOTSTRAP_PATH}`);

function runScript(command: string, args: string[]): void {
  const child = spawnSync(command, args, {
    cwd: PROJECT_ROOT,
    stdio: 'inherit',
    shell: process.platform === 'win32',
  });
  if (child.status !== 0) {
    throw new Error(
      `Command "${command} ${args.join(' ')}" failed with code ${child.status ?? 'unknown'}`,
    );
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

async function isPortInUse(port: number): Promise<boolean> {
  return await new Promise((resolve) => {
    const socket = createConnection({ host: '127.0.0.1', port });
    socket.once('connect', () => {
      socket.end();
      resolve(true);
    });
    socket.once('error', () => resolve(false));
  });
}

async function ensureDriverReady(): Promise<void> {
  const proxyPort = Number(process.env.TAURI_DRIVER_PORT ?? '4445');
  if (await isPortInUse(proxyPort)) {
    return;
  }
  console.warn(
    `[wdio.desktop] tauri-driver proxy not responding on ${proxyPort}; attempting restart`,
  );
  await startDriver();
}

function seedCliBootstrapFixture(): void {
  const payload = {
    nodes: ['node1@127.0.0.1:11223', 'node2@127.0.0.1:11224'],
    updated_at_ms: Date.now(),
  };
  mkdirSync(dirname(CLI_BOOTSTRAP_PATH), { recursive: true });
  writeFileSync(CLI_BOOTSTRAP_PATH, JSON.stringify(payload, null, 2), 'utf-8');
  console.info(`[wdio.desktop] wrote CLI bootstrap fixture to ${CLI_BOOTSTRAP_PATH}`);
}

export const config: Options.Testrunner = {
  runner: 'local',
  workers: WORKER_COUNT,
  specs: [join(__dirname, 'specs/**/*.spec.ts')],
  maxInstances: WORKER_COUNT,
  logLevel: 'info',
  waitforTimeout: 15000,
  connectionRetryTimeout: 120000,
  connectionRetryCount: 2,
  port: Number(process.env.TAURI_DRIVER_PORT ?? '4445'),
  autoCompileOpts: {
    autoCompile: true,
    tsNodeOpts: {
      project: './tests/e2e/tsconfig.json',
      transpileOnly: true,
      require: ['tsconfig-paths/register'],
    },
  },
  reporters: [
    'spec',
    [
      '@wdio/spec-reporter',
      {
        showPreface: false,
      },
    ],
  ],
  framework: 'mocha',
  mochaOpts: {
    ui: 'bdd',
    timeout: 60000,
  },
  hostname: '127.0.0.1',
  capabilities: [
    {
      maxInstances: WORKER_COUNT,
      browserName: 'wry',
      'wdio:enforceWebDriverClassic': true,
      'tauri:options': {
        application:
          process.env.TAURI_E2E_APP_PATH ??
          join(PROJECT_ROOT, 'src-tauri', 'target', 'debug', binaryName()),
      },
    },
  ],
  onPrepare: async (_config, capabilities) => {
    mkdirSync(OUTPUT_DIR, { recursive: true });
    seedCliBootstrapFixture();
    let baseUrl = process.env.E2E_COMMUNITY_NODE_URL;
    if (!baseUrl) {
      const result = await startCommunityNodeMock();
      baseUrl = result.baseUrl;
      process.env.E2E_COMMUNITY_NODE_URL = baseUrl;
      shouldStopCommunityNodeMock = true;
      console.info(`[wdio.desktop] community node mock running at ${baseUrl}`);
    } else {
      console.info(`[wdio.desktop] community node base URL preset to ${baseUrl}`);
    }
    if (Array.isArray(capabilities)) {
      for (const capability of capabilities) {
        pruneUnsupportedCapabilities(capability as Record<string, unknown>);
      }
    }
    if (process.env.E2E_SKIP_BUILD !== '1') {
      runScript('pnpm', ['e2e:build']);
    }
    await startDriver();
  },
  beforeSession: async function (_config, capabilities) {
    pruneUnsupportedCapabilities(capabilities);
    await ensureDriverReady();
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
    try {
      if (shouldStopCommunityNodeMock) {
        await stopCommunityNodeMock();
      }
    } finally {
      await stopDriver();
    }
  },
};
