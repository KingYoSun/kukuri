import { spawn, type ChildProcess } from 'node:child_process';
import { createServer, request } from 'node:http';
import { access } from 'node:fs/promises';
import { constants as fsConstants, existsSync } from 'node:fs';
import { homedir, platform } from 'node:os';
import { join, resolve } from 'node:path';
import { createConnection } from 'node:net';

let driverProcess: ChildProcess | null = null;
let proxyServer: ReturnType<typeof createServer> | null = null;
const DEFAULT_PORT = process.env.TAURI_DRIVER_PORT ?? '4445';
const DRIVER_READY_TIMEOUT_MS = Number(process.env.TAURI_DRIVER_READY_TIMEOUT ?? '15000');
const DRIVER_READY_POLL_MS = 200;

async function ensureExecutable(binaryPath: string): Promise<void> {
  try {
    await access(binaryPath, fsConstants.X_OK);
  } catch {
    throw new Error(
      `tauri-driver binary not found or not executable at ${binaryPath}. ` +
        'Install it via "cargo install tauri-driver --locked" or set TAURI_DRIVER_BINARY.',
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
      'Install it via "cargo install tauri-driver --locked" or set TAURI_DRIVER_BINARY.',
  );
}

function resolveNativeDriver(): string | undefined {
  if (process.env.TAURI_NATIVE_DRIVER) {
    return process.env.TAURI_NATIVE_DRIVER;
  }

  if (platform() === 'win32') {
    const candidate = resolve(process.cwd(), 'msedgedriver.exe');
    return existsSync(candidate) ? candidate : undefined;
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

  const resolvedPort = Number(process.env.TAURI_DRIVER_PORT ?? DEFAULT_PORT);
  console.info(`[tauriDriver] starting with port=${resolvedPort}`);
  const binaryPath = await resolveDriverBinary();
  const isLinux = platform() === 'linux';
  let proxyListenPort = resolvedPort;
  const nativeHost = process.env.TAURI_NATIVE_DRIVER_HOST ?? '127.0.0.1';
  let nativePort = Number(
    process.env.TAURI_NATIVE_DRIVER_PORT ?? (isLinux ? proxyListenPort + 100 : 4444),
  );
  const nativeDriverPath = resolveNativeDriver();

  if (isLinux) {
    if (proxyListenPort === nativePort) {
      proxyListenPort += 2;
    }
    // avoid binding failures if the native port is already taken
    let attempts = 0;
    while (attempts < 5 && (await isPortInUse(nativePort))) {
      nativePort += 2;
      attempts += 1;
    }
  }
  const driverPort = isLinux ? proxyListenPort + 1 : proxyListenPort;

  if (
    (await isPortInUse(proxyListenPort)) ||
    (isLinux && (await isPortInUse(driverPort))) ||
    (isLinux && (await isPortInUse(nativePort)))
  ) {
    console.warn(
      `tauri-driver ports already in use (proxy=${proxyListenPort}, driver=${driverPort}, native=${nativePort}); assuming existing driver`,
    );
    return;
  }

  if (isLinux) {
    startCapabilityProxy(proxyListenPort, driverPort);
  }

  const args = ['--port', driverPort.toString()];
  if (nativeDriverPath) {
    args.push('--native-driver', nativeDriverPath);
    if (platform() === 'linux') {
      args.push('--native-host', nativeHost, '--native-port', nativePort.toString());
    }
  }

  try {
    console.info(`[tauriDriver] spawning ${binaryPath} ${args.join(' ')}`);
    driverProcess = spawn(binaryPath, args, {
      stdio: 'inherit',
    });
  } catch (error) {
    console.warn('[tauriDriver] failed to spawn driver', error);
    driverProcess = null;
    throw error;
  }

  driverProcess.once('exit', (code, signal) => {
    if (code !== 0) {
      console.warn(`tauri-driver exited unexpectedly (code=${code}, signal=${signal})`);
    }
    driverProcess = null;
  });

  if (isLinux && nativeDriverPath) {
    await waitForPortReady(nativePort, DRIVER_READY_TIMEOUT_MS);
  }
  if (isLinux) {
    await waitForPortReady(proxyListenPort, DRIVER_READY_TIMEOUT_MS);
  }
  await waitForPortReady(driverPort, DRIVER_READY_TIMEOUT_MS);
  await waitForDriverStatus(isLinux ? proxyListenPort : driverPort, DRIVER_READY_TIMEOUT_MS);
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

      if (req.method === 'POST' && req.url && req.url.startsWith('/session') && body.length > 0) {
        try {
          const json = JSON.parse(body.toString('utf-8'));
          pruneCapabilityPayload(json);
          const serialized = Buffer.from(JSON.stringify(json), 'utf-8');
          headers['content-length'] = Buffer.byteLength(serialized).toString();
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
          headers,
        },
        (proxyRes) => {
          res.writeHead(proxyRes.statusCode ?? 502, proxyRes.headers);
          proxyRes.pipe(res);
        },
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

async function waitForPortReady(port: number, timeoutMs: number): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (await isPortInUse(port)) {
      return;
    }
    await new Promise((resolve) => setTimeout(resolve, DRIVER_READY_POLL_MS));
  }
  throw new Error(`tauri-driver did not listen on port ${port} within ${timeoutMs}ms`);
}

async function waitForDriverStatus(port: number, timeoutMs: number): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const isReady = await new Promise<boolean>((resolve) => {
      const req = request(
        {
          hostname: '127.0.0.1',
          port,
          path: '/status',
          method: 'GET',
        },
        (res) => {
          const chunks: Buffer[] = [];
          res.on('data', (chunk) => chunks.push(chunk as Buffer));
          res.on('end', () => {
            if (!res.statusCode || res.statusCode < 200 || res.statusCode >= 300) {
              resolve(false);
              return;
            }
            if (chunks.length === 0) {
              resolve(true);
              return;
            }
            try {
              const payload = JSON.parse(Buffer.concat(chunks).toString('utf-8')) as {
                value?: { ready?: boolean };
              };
              if (payload?.value?.ready === false) {
                resolve(false);
                return;
              }
            } catch {
              // Assume ready when a successful response does not contain JSON.
            }
            resolve(true);
          });
        },
      );

      req.on('error', () => resolve(false));
      req.end();
    });

    if (isReady) {
      return;
    }
    await new Promise((resolve) => setTimeout(resolve, DRIVER_READY_POLL_MS));
  }
  throw new Error(`tauri-driver did not respond to /status on port ${port} within ${timeoutMs}ms`);
}

function pruneCapabilityPayload(payload: unknown): void {
  if (payload && typeof payload === 'object' && 'capabilities' in payload) {
    pruneUnsupportedCapabilities((payload as { capabilities?: unknown }).capabilities);
  }
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
