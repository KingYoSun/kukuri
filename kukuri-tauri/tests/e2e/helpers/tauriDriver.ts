import { spawn, type ChildProcess } from 'node:child_process';
import { createServer, request } from 'node:http';
import { access } from 'node:fs/promises';
import { constants as fsConstants, existsSync } from 'node:fs';
import { homedir, platform } from 'node:os';
import { join, resolve } from 'node:path';

let driverProcess: ChildProcess | null = null;
let proxyServer: ReturnType<typeof createServer> | null = null;
const DEFAULT_PORT = process.env.TAURI_DRIVER_PORT ?? '4445';

async function ensureExecutable(binaryPath: string): Promise<void> {
  try {
    await access(binaryPath, fsConstants.X_OK);
  } catch {
    throw new Error(
      `tauri-driver binary not found or not executable at ${binaryPath}. ` +
        'Install it via "cargo install tauri-driver --locked" or set TAURI_DRIVER_BINARY.'
    );
  }
}

async function resolveDriverBinary(): Promise<string> {
  const isWindows = platform() === 'win32';
  const fileName = isWindows ? 'tauri-driver.exe' : 'tauri-driver';
  const candidates: string[] = [];

  if (process.env.TAURI_DRIVER_BINARY) {
    candidates.push(process.env.TAURI_DRIVER_BINARY);
  }
  if (process.env.CARGO_HOME) {
    candidates.push(join(process.env.CARGO_HOME, 'bin', fileName));
  }

  candidates.push(join(homedir(), '.cargo', 'bin', fileName));

  for (const candidate of candidates) {
    try {
      await ensureExecutable(candidate);
      return candidate;
    } catch {
      continue;
    }
  }

  throw new Error(
    `tauri-driver binary not found. Checked paths: ${candidates.join(', ')}. ` +
      'Install it via "cargo install tauri-driver --locked" or set TAURI_DRIVER_BINARY.'
  );
}

function resolveNativeDriver(): string | undefined {
  if (process.env.TAURI_NATIVE_DRIVER) {
    return process.env.TAURI_NATIVE_DRIVER;
  }

  if (platform() === 'win32') {
    return resolve(process.cwd(), 'msedgedriver.exe');
  }

  if (platform() === 'linux') {
    const linuxDriver = '/usr/bin/WebKitWebDriver';
    if (existsSync(linuxDriver)) {
      return linuxDriver;
    }
  }

  return undefined;
}

export async function startDriver(): Promise<void> {
  if (driverProcess) {
    return;
  }

  const binaryPath = await resolveDriverBinary();
  const isLinux = platform() === 'linux';
  const proxyListenPort = Number(DEFAULT_PORT);
  const driverPort = isLinux ? proxyListenPort + 1 : proxyListenPort;

  if (isLinux) {
    startCapabilityProxy(proxyListenPort, driverPort);
  }

  const args = ['--port', driverPort.toString()];
  const nativeDriverPath = resolveNativeDriver();
  if (nativeDriverPath) {
    args.push('--native-driver', nativeDriverPath);
    if (platform() === 'linux') {
      const nativeHost = process.env.TAURI_NATIVE_DRIVER_HOST ?? '127.0.0.1';
      const nativePort = process.env.TAURI_NATIVE_DRIVER_PORT ?? '4444';
      args.push('--native-host', nativeHost, '--native-port', nativePort);
    }
  }

  driverProcess = spawn(binaryPath, args, {
    stdio: 'inherit'
  });

  driverProcess.once('exit', (code, signal) => {
    if (code !== 0) {
      console.warn(`tauri-driver exited unexpectedly (code=${code}, signal=${signal})`);
    }
    driverProcess = null;
  });
}

export async function stopDriver(): Promise<void> {
  if (!driverProcess) {
    return;
  }
  driverProcess.kill('SIGTERM');
  driverProcess = null;

  if (proxyServer) {
    proxyServer.close();
    proxyServer = null;
  }
}

function startCapabilityProxy(listenPort: number, targetPort: number): void {
  if (proxyServer) {
    return;
  }

  proxyServer = createServer((req, res) => {
    const chunks: Buffer[] = [];
    req.on('data', (chunk) => chunks.push(chunk as Buffer));
    req.on('end', () => {
      let body = Buffer.concat(chunks);
      const headers = { ...req.headers };

      if (
        req.method === 'POST' &&
        req.url &&
        req.url.startsWith('/session') &&
        body.length > 0
      ) {
        try {
          const json = JSON.parse(body.toString('utf-8'));
          pruneCapabilityPayload(json);
          const serialized = Buffer.from(JSON.stringify(json), 'utf-8');
          headers['content-length'] = Buffer.byteLength(
            serialized
          ).toString();
          body = serialized;
        } catch (error) {
          res.statusCode = 400;
          res.end(`Invalid capability payload: ${(error as Error).message}`);
          return;
        }
      }

      const proxyReq = request(
        {
          hostname: '127.0.0.1',
          port: targetPort,
          path: req.url,
          method: req.method,
          headers
        },
        (proxyRes) => {
          res.writeHead(proxyRes.statusCode ?? 502, proxyRes.headers);
          proxyRes.pipe(res);
        }
      );

      proxyReq.on('error', (error) => {
        res.statusCode = 502;
        res.end(`Proxy request failed: ${error.message}`);
      });

      if (body.length > 0) {
        proxyReq.write(body);
      }
      proxyReq.end();
    });
  });

  proxyServer.listen(listenPort, '127.0.0.1');
}

function pruneCapabilityPayload(payload: unknown): void {
  if (
    payload &&
    typeof payload === 'object' &&
    'capabilities' in payload
  ) {
    pruneUnsupportedCapabilities((payload as { capabilities?: unknown }).capabilities);
  }
}

function pruneUnsupportedCapabilities(target: unknown): void {
  if (!target || typeof target !== 'object') {
    return;
  }

  const record = target as Record<string, unknown>;
  if ('webSocketUrl' in record) {
    // eslint-disable-next-line @typescript-eslint/no-dynamic-delete
    delete record.webSocketUrl;
  }
  if ('unhandledPromptBehavior' in record) {
    // eslint-disable-next-line @typescript-eslint/no-dynamic-delete
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
