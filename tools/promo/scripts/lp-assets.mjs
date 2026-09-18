import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import process from 'node:process';

/**
 * LP (apps/lp) が配信する画像を、撮影済みの原素材から作る (#1043)。
 *
 * - 画面の静止画: 原素材に「デモ画面」「実機」の表記と、実験機能なら必須の表記を重ねる。
 *   LP では見出しと説明を本文に書くので、字幕は焼き込まない。
 * - OGP: 1200×630。Dome は入れない (brief)。
 *
 * 先に撮影 (apps/desktop の playwright.promo.config.ts) と、実機の静止画の取り込み
 * (scripts/import-still.mjs) を済ませておく。
 *
 *   node scripts/lp-assets.mjs
 */

const PROMO_ROOT = path.resolve(import.meta.dirname, '..');
const ARTIFACTS = path.resolve(PROMO_ROOT, '../../promo-artifacts');
const OUT = path.resolve(PROMO_ROOT, '../../apps/lp/public/assets/screens');
const REMOTION = path.join(PROMO_ROOT, 'node_modules/@remotion/cli/remotion-cli.js');

/** LP の場面と原素材の対応。brief の固定出力一覧 (AC-3) に合わせる。 */
const SCREENS = [
  { name: 'hero', capture: 's0-hero/c1' },
  { name: 'scene1', capture: 's1-topic/c1' },
  { name: 'scene2', capture: 's2-conversation/c2' },
  { name: 'scene3', capture: 's3-private-channel/c2' },
];
const LOCALES = ['ja', 'en'];
const DOME = { name: 'dome-teaser', capture: 's9-dome-teaser/c1', locale: 'ja' };

const OGP_COPY = {
  ja: {
    title: '話題からつながる、\nあなたの端末が起点のSNS',
    subtitle: '話題を選び、公開で語り、同じ話題の中で小さな輪へ。',
    demoLabel: 'デモ画面',
  },
  en: {
    title: 'A topic-first social app that starts on your device',
    subtitle: 'Pick a topic, talk in the open, move into a smaller circle.',
    demoLabel: 'Demo screen',
  },
};

const tmp = path.join(ARTIFACTS, 'lp-props');
mkdirSync(tmp, { recursive: true });
mkdirSync(OUT, { recursive: true });

function readProps(capture, locale) {
  const file = path.join(ARTIFACTS, 'captures', capture, `${locale}-dark`, 'props.json');
  if (!existsSync(file)) {
    throw new Error(`原素材が無い: ${file}。先に撮影または取り込みを行う`);
  }
  return JSON.parse(readFileSync(file, 'utf8'));
}

function still(composition, props, output) {
  const propsFile = path.join(tmp, `${path.basename(output)}.json`);
  writeFileSync(propsFile, JSON.stringify(props), 'utf8');
  execFileSync(
    process.execPath,
    [REMOTION, 'still', 'src/index.ts', composition, output, `--props=${propsFile}`, '--log=error'],
    { cwd: PROMO_ROOT, stdio: 'inherit' }
  );
  process.stdout.write(`wrote ${path.relative(process.cwd(), output)}\n`);
}

for (const locale of LOCALES) {
  for (const screen of SCREENS) {
    const props = readProps(screen.capture, locale);
    still('SceneStill', { ...props, caption: null }, path.join(OUT, `${screen.name}-${locale}.png`));
  }
  const hero = readProps('s0-hero/c1', locale);
  still(
    'Ogp',
    { still: hero.manifest.files.still, ...OGP_COPY[locale] },
    path.join(OUT, `ogp-${locale}.png`)
  );
}

// Dome の予告は日本語 UI の実機画面 1 枚だけ。画面の説明は LP の本文に書く。
const dome = readProps(DOME.capture, DOME.locale);
still('SceneStill', { ...dome, caption: null }, path.join(OUT, `${DOME.name}.png`));

rmSync(tmp, { recursive: true, force: true });
