import { rmSync } from 'node:fs';
import path from 'node:path';
import process from 'node:process';

/**
 * 撮影・レンダリングの出力を消す。
 *
 * 既定では render 出力と Playwright の artifact だけを消し、撮り直しに時間のかかる
 * 原素材 (captures) は残す。原素材も消す場合は --captures を付ける。
 *
 *   node scripts/clean.mjs
 *   node scripts/clean.mjs --captures
 */

const root = path.resolve(import.meta.dirname, '../../../promo-artifacts');
const withCaptures = process.argv.includes('--captures');

const targets = [
  path.join(root, 'renders'),
  path.join(root, 'playwright-output'),
  ...(withCaptures ? [path.join(root, 'captures')] : []),
];

for (const target of targets) {
  rmSync(target, { recursive: true, force: true });
  process.stdout.write(`removed: ${target}\n`);
}

if (!withCaptures) {
  process.stdout.write('原素材 (captures) は残した。消す場合は --captures を付ける。\n');
}
