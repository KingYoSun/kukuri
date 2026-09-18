import { expect, type Browser, type Locator, type Page } from '@playwright/test';
import { writeFileSync } from 'node:fs';
import path from 'node:path';

import {
  captureDir,
  finalizeVideo,
  prepareCaptureDir,
  toPublicRelative,
  writeManifest,
  type CaptureTarget,
} from '../promoArtifacts';

/**
 * 1 カットの撮影手順 (#1039)。
 *
 * 画面が描き終わる前の状態を素材として採用しないよう、撮影の直前に
 * 「文字・画像・フォントが出そろったか」を必ず確認する。networkidle や
 * 固定 sleep だけを完了条件にしない。
 */

export type SceneStep = {
  /** 場面と cut の識別子。brief の shot list と対応させる。 */
  target: CaptureTarget;
  /** 開くページ。 */
  url: string;
  /** 撮影対象に必ず映っている要素。これが見えるまで撮影を始めない。 */
  anchor: (page: Page) => Locator;
  /** 撮影前に済ませておく操作（録画には入れない）。 */
  prepare?: (page: Page) => Promise<void>;
  /** 録画中に行う操作。省略すると静止画だけを撮る。 */
  act?: (page: Page) => Promise<void>;
  /** 焼き込む字幕。 */
  caption: string | null;
};

const READY_TIMEOUT_MS = 15_000;

/**
 * フォントと画像が出そろうまで待つ。上限を過ぎたら失敗させ、
 * 未描画の画面を成功として採用しない (TR-3)。
 */
export async function waitForRenderReady(page: Page) {
  await page.waitForFunction(() => document.fonts.status === 'loaded', undefined, {
    timeout: READY_TIMEOUT_MS,
  });

  const pending = await page.evaluate(() =>
    Array.from(document.images)
      .filter((image) => !image.complete || image.naturalWidth === 0)
      .map((image) => image.currentSrc || image.src)
  );
  if (pending.length > 0) {
    throw new Error(`promo capture: 読み込みが終わっていない画像がある: ${pending.join(', ')}`);
  }

  // 文字が 1 つも描かれていない画面を撮らない。
  const text = await page.evaluate(() => document.body.innerText.trim().length);
  if (text === 0) {
    throw new Error('promo capture: 画面に文字が無い');
  }
}

const STABLE_ATTEMPTS = 20;
const STABLE_INTERVAL_MS = 150;

/**
 * 画面が描き変わらなくなるまで待ち、その画面の PNG を返す。
 *
 * 文字と画像がそろっても、プロフィールなどの非同期の読み込みが続いていると、
 * 撮るたびに少しずつ違う画面になる。連続 2 回の撮影が完全に一致するまで待ち、
 * 上限までに落ち着かなければ失敗させる (TR-3)。
 */
export async function captureStableScreen(page: Page): Promise<Buffer> {
  const shoot = () => page.screenshot({ animations: 'disabled', caret: 'hide' });
  let previous = await shoot();
  for (let attempt = 0; attempt < STABLE_ATTEMPTS; attempt += 1) {
    await page.waitForTimeout(STABLE_INTERVAL_MS);
    const current = await shoot();
    if (current.equals(previous)) {
      return current;
    }
    previous = current;
  }
  throw new Error(
    `promo capture: ${STABLE_ATTEMPTS * STABLE_INTERVAL_MS}ms 待っても画面が落ち着かない`
  );
}

/** 文字化けの検出。合成データに無いはずの置換文字が出ていたら失敗させる。 */
export async function assertNoBrokenGlyphs(page: Page) {
  const broken = await page.evaluate(() => {
    const body = document.body.innerText;
    // U+FFFD (置換文字) の個数。ソースに置換文字そのものを書かないよう、コードポイントで作る。
    const replacement = body.split(String.fromCharCode(0xfffd)).length - 1;
    return { replacement, sample: body.slice(0, 200) };
  });
  if (broken.replacement > 0) {
    throw new Error(
      `promo capture: 置換文字 (U+FFFD) が ${broken.replacement} 個ある。フォントか文字コードを確認する`
    );
  }
}

export async function captureSceneStep(page: Page, browser: Browser, step: SceneStep) {
  const dir = prepareCaptureDir(step.target);

  await page.goto(step.url);
  await expect(step.anchor(page)).toBeVisible();
  if (step.prepare) {
    await step.prepare(page);
  }
  await waitForRenderReady(page);
  await assertNoBrokenGlyphs(page);

  // 静止画は動きを止め、入力欄の点滅するカーソルも隠し、画面が落ち着いてから撮る。
  // 操作 clip では止めない。
  const stillPath = path.join(dir, 'still.png');
  writeFileSync(stillPath, await captureStableScreen(page));

  let videoRelative: string | null = null;
  let clip: { startMs: number; endMs: number } | null = null;

  if (step.act) {
    const rawVideoPath = path.join(dir, 'raw.webm');
    const startedAt = Date.now();
    // size は viewport と同値で明示する。省略すると録画が縮小される。
    await page.screencast.start({ path: rawVideoPath, size: step.target.viewport });
    await step.act(page);
    await page.screencast.stop();
    clip = { startMs: 0, endMs: Date.now() - startedAt };
    videoRelative = toPublicRelative(finalizeVideo(rawVideoPath, dir));
  }

  const manifestPath = writeManifest(
    step.target,
    {
      videoRelative,
      stillRelative: toPublicRelative(stillPath),
      clip,
      platform: {
        os: `${process.platform} ${process.arch}`,
        browser: browser.browserType().name(),
        browserVersion: browser.version(),
      },
    },
    { caption: step.caption }
  );

  return { dir: captureDir(step.target), manifestPath };
}
