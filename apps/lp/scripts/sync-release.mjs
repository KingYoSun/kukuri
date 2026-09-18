import { readFileSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import process from 'node:process';

/**
 * LP が案内する release を `apps/lp/release.json` の 1 か所で管理する (#1043)。
 *
 * LP は依存もビルドも無い静的ファイルなので、release の tag・配布物の名前・版表記は
 * 各 HTML に直接書いてある。このスクリプトは release.json に合わせてそれらを書き換え、
 * `--check` では書き換えずに食い違いだけを報告して失敗する。
 *
 *   node apps/lp/scripts/sync-release.mjs          # 反映
 *   node apps/lp/scripts/sync-release.mjs --check  # 検査のみ
 */

const LP = path.resolve(import.meta.dirname, '..');
const PAGES = ['public/index.html', 'public/en/index.html'];

const release = JSON.parse(readFileSync(path.join(LP, 'release.json'), 'utf8'));
if (!/^v\d+\.\d+\.\d+-preview\.\d+$/.test(release.tag) || !/^\d+\.\d+\.\d+$/.test(release.version)) {
  process.stderr.write('sync-release: release.json の tag / version の形が不正\n');
  process.exit(1);
}
if (!release.tag.startsWith(`v${release.version}-`)) {
  process.stderr.write(`sync-release: tag ${release.tag} と version ${release.version} が食い違う\n`);
  process.exit(1);
}

const TAG = /v\d+\.\d+\.\d+-preview\.\d+/g;
// 配布物の名前 (kukuri_0.2.6_x64-setup.exe、kukuri-cli_0.2.6_x86_64-... など) の版の部分。
const ASSET = /(kukuri(?:-cli)?_)\d+\.\d+\.\d+(_)/g;

const check = process.argv.includes('--check');
let mismatches = 0;

for (const page of PAGES) {
  const file = path.join(LP, page);
  const before = readFileSync(file, 'utf8');
  const after = before
    .replace(TAG, release.tag)
    .replace(ASSET, (_, head, tail) => `${head}${release.version}${tail}`);

  if (before === after) {
    process.stdout.write(`ok: ${page} は ${release.tag} に一致\n`);
    continue;
  }
  const stale = new Set([...(before.match(TAG) ?? []), ...(before.match(ASSET) ?? [])]);
  if (check) {
    mismatches += 1;
    process.stderr.write(`mismatch: ${page} に release.json と違う版がある: ${[...stale].join(', ')}\n`);
  } else {
    writeFileSync(file, after, 'utf8');
    process.stdout.write(`updated: ${page} -> ${release.tag}\n`);
  }
}

if (mismatches > 0) {
  process.exit(1);
}
