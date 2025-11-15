import { spawn, type ChildProcess } from 'node:child_process';
import { access } from 'node:fs/promises';
import { constants as fsConstants } from 'node:fs';
import { homedir, platform } from 'node:os';
import { join, resolve } from 'node:path';

let driverProcess: ChildProcess | null = null;
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

function resolveDriverBinary(): string {
  if (process.env.TAURI_DRIVER_BINARY) {
    return process.env.TAURI_DRIVER_BINARY;
  }
  const isWindows = platform() === 'win32';
  const fileName = isWindows ? 'tauri-driver.exe' : 'tauri-driver';
  return join(homedir(), '.cargo', 'bin', fileName);
}

function resolveNativeDriver(): string | undefined {
  if (process.env.TAURI_NATIVE_DRIVER) {
    return process.env.TAURI_NATIVE_DRIVER;
  }
  if (platform() === 'win32') {
    const candidate = resolve(process.cwd(), 'msedgedriver.exe');
    return candidate;
  }
  return undefined;
}

export async function startDriver(): Promise<void> {
  if (driverProcess) {
    return;
  }

  const binaryPath = resolveDriverBinary();
  await ensureExecutable(binaryPath);

  const args = ['--port', DEFAULT_PORT];
  const nativeDriverPath = resolveNativeDriver();
  if (nativeDriverPath) {
    args.push('--native-driver', nativeDriverPath);
  }

  driverProcess = spawn(binaryPath, args, {
    stdio: 'inherit'
  });

  driverProcess.once('exit', (code, signal) => {
    if (code !== 0) {
      console.error(`tauri-driver exited unexpectedly (code=${code}, signal=${signal})`);
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
}
