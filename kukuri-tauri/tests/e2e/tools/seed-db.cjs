#!/usr/bin/env node
/* eslint-env node */
const fs = require('fs/promises');
const path = require('path');

const projectRoot = path.resolve(__dirname, '..', '..');
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

async function copySeed() {
  const hasSeed = await fileExists(seedDb);
  if (hasSeed) {
    await fs.copyFile(seedDb, targetDb);
    console.log(`[E2E seed] Copied fixture database from ${seedDb}`);
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
