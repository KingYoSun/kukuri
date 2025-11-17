export async function readFile(): Promise<unknown> {
  throw new Error('Mock for @tauri-apps/plugin-fs.readFile is not configured');
}

export async function readTextFile(): Promise<string> {
  throw new Error('Mock for @tauri-apps/plugin-fs.readTextFile is not configured');
}

export async function writeTextFile(): Promise<void> {
  throw new Error('Mock for @tauri-apps/plugin-fs.writeTextFile is not configured');
}
