#!/usr/bin/env node
import { promises as fs } from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const repoRoot = path.resolve(__dirname, '..');
const commandsDir = path.join(repoRoot, 'kukuri-tauri', 'src-tauri', 'src', 'presentation', 'commands');
const frontendDir = path.join(repoRoot, 'kukuri-tauri', 'src');

async function listFiles(dir, extensions) {
  const entries = await fs.readdir(dir, { withFileTypes: true });
  const files = await Promise.all(
    entries.map(async (entry) => {
      const fullPath = path.join(dir, entry.name);
      if (entry.isDirectory()) {
        return listFiles(fullPath, extensions);
      }
      if (!extensions.some((ext) => entry.name.endsWith(ext))) {
        return [];
      }
      return [fullPath];
    }),
  );
  return files.flat();
}

async function collectTauriCommands() {
  const files = await listFiles(commandsDir, ['.rs']);
  const commandPattern = /#\[tauri::command[^\]]*\]\s*(?:pub\s+)?(?:async\s+)?fn\s+([a-zA-Z0-9_]+)/g;
  const commands = new Set();
  for (const file of files) {
    const content = await fs.readFile(file, 'utf-8');
    for (const match of content.matchAll(commandPattern)) {
      commands.add(match[1]);
    }
  }
  return commands;
}

async function collectFrontendInvocations() {
  const files = await listFiles(frontendDir, ['.ts', '.tsx', '.mts', '.cts', '.js', '.jsx']);
  const invokePattern = /(invokeCommand(?:Void)?|invoke)[^\(]*\(\s*['"]([a-zA-Z0-9_]+)['"]/g;
  const commands = new Set();
  for (const file of files) {
    const content = await fs.readFile(file, 'utf-8');
    for (const match of content.matchAll(invokePattern)) {
      commands.add(match[2]);
    }
  }
  return commands;
}

(async () => {
  const [definedCommands, invokedCommands] = await Promise.all([
    collectTauriCommands(),
    collectFrontendInvocations(),
  ]);

  const missing = Array.from(definedCommands).filter((name) => !invokedCommands.has(name));
  if (missing.length > 0) {
    console.error('未接続の Tauri コマンドが見つかりました:\n');
    for (const name of missing.sort()) {
      console.error(` - ${name}`);
    }
    process.exit(1);
  }

  console.log(`Tauri コマンド ${definedCommands.size} 件はすべてフロントエンドから呼び出されています。`);
})();
