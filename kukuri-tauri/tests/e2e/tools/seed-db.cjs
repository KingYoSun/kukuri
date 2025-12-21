#!/usr/bin/env node
/* eslint-env node */
const fs = require('fs/promises');
const os = require('os');
const path = require('path');

const projectRoot = path.resolve(__dirname, '..', '..', '..');
const dataDir = path.join(projectRoot, 'src-tauri', 'data');
const targetDb = path.join(dataDir, 'kukuri.db');
const seedDb = path.join(projectRoot, 'testdata', 'e2e_seed.db');

async function fileExists(filePath) {
  try {
    await fs.access(filePath);
    return true;
  } catch {
    return false;
  }
}

async function ensureDataDirectory() {
  await fs.mkdir(dataDir, { recursive: true });
}

async function resolveAppDataDir() {
  const configPath = path.join(projectRoot, 'src-tauri', 'tauri.conf.json');
  let identifier = 'com.kukuri.app';
  try {
    const raw = await fs.readFile(configPath, 'utf8');
    const parsed = JSON.parse(raw);
    if (typeof parsed?.identifier === 'string' && parsed.identifier.trim()) {
      identifier = parsed.identifier.trim();
    }
  } catch {
    // ignore and use fallback identifier
  }

  const homeDir = os.homedir();
  let baseDir = '';
  if (process.platform === 'win32') {
    baseDir = process.env.APPDATA || path.join(homeDir, 'AppData', 'Roaming');
  } else if (process.platform === 'darwin') {
    baseDir = path.join(homeDir, 'Library', 'Application Support');
  } else {
    baseDir = process.env.XDG_DATA_HOME || path.join(homeDir, '.local', 'share');
  }

  return path.join(baseDir, identifier);
}

async function copySeed() {
  const hasSeed = await fileExists(seedDb);
  if (hasSeed) {
    await fs.copyFile(seedDb, targetDb);
    console.log(`[E2E seed] Copied fixture database from ${seedDb}`);
    const appDataDir = await resolveAppDataDir();
    const appDataDb = path.join(appDataDir, 'kukuri.db');
    await fs.mkdir(appDataDir, { recursive: true });
    await fs.copyFile(seedDb, appDataDb);
    console.log(`[E2E seed] Copied fixture database to ${appDataDb}`);
    return;
  }

  if (await fileExists(targetDb)) {
    await fs.unlink(targetDb);
  }
  await fs.writeFile(targetDb, '');
  console.warn(
    '[E2E seed] No seed database found. Created an empty placeholder at src-tauri/data/kukuri.db.'
  );
}

async function main() {
  await ensureDataDirectory();
  await copySeed();
}

main().catch((error) => {
  console.error('[E2E seed] Failed to prepare database:', error);
  process.exit(1);
});
