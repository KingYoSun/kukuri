import { expect, test } from '@playwright/test';
import path from 'node:path';

import { DEVELOPER_MODE_STORAGE_KEY } from '../../src/lib/developerMode';
import {
  finalizeVideo,
  prepareCaptureDir,
  toPublicRelative,
  writeManifest,
  type CaptureTarget,
} from './promoArtifacts';

/**
 * 制作環境の最小経路 (#1038)。1 scene 分の PNG と短い MP4 の素材を取得する。
 *
 * 台本に沿った 3 場面の撮影は #1039 が所有する。ここは環境が通ることだけを確かめる
 * ため、既定で表示されるタイムラインを撮る。開発者モードは無効のままにする。
 */

const VIEWPORT = { width: 1600, height: 1000 };

const TARGET: CaptureTarget = {
  sceneId: 'smoke',
  cutId: 'c1',
  locale: 'ja',
  theme: 'dark',
  developerMode: false,
  viewport: VIEWPORT,
};

// 録画の長さ。AC-1 の 5〜10 秒に収める。
const CLIP_MS = 6000;

test.use({ viewport: VIEWPORT });

test('promo smoke: 1 scene の still と clip を取得する', async ({ page, browser }) => {
  const dir = prepareCaptureDir(TARGET);

  await page.addInitScript(
    ({ locale, theme, developerModeKey, developerMode }) => {
      localStorage.setItem('kukuri.desktop.locale', locale);
      localStorage.setItem('kukuri.desktop.theme', theme);
      localStorage.setItem(developerModeKey, developerMode ? 'true' : 'false');
    },
    {
      locale: TARGET.locale,
      theme: TARGET.theme,
      developerModeKey: DEVELOPER_MODE_STORAGE_KEY,
      developerMode: TARGET.developerMode,
    }
  );

  await page.goto('/');

  const columns = page.locator('[data-column-id]');
  await expect(columns.first()).toBeVisible();

  // 実験機能の面が既定で出ていないことを、撮影の前提として確認する。
  await expect(page.locator('[data-column-id][aria-label^="Metaverse"]')).toHaveCount(0);

  const stillPath = path.join(dir, 'still.png');
  await page.screenshot({ path: stillPath });

  const rawVideoPath = path.join(dir, 'raw.webm');
  const startedAt = Date.now();
  // size を viewport と同値で明示する。省略すると既定の大きさへ縮小された動画になる。
  await page.screencast.start({ path: rawVideoPath, size: VIEWPORT });

  // 画面が動いていることが分かる最小の操作。列の移動だけに留め、投稿は作らない。
  const columnCount = await columns.count();
  for (let index = 0; index < Math.min(columnCount, 3); index += 1) {
    await columns.nth(index).click({ position: { x: 24, y: 24 } });
    await page.waitForTimeout(Math.floor(CLIP_MS / 3));
  }

  await page.screencast.stop();
  const endedAt = Date.now();

  const videoPath = finalizeVideo(rawVideoPath, dir);

  writeManifest(TARGET, {
    videoRelative: toPublicRelative(videoPath),
    stillRelative: toPublicRelative(stillPath),
    clip: { startMs: 0, endMs: endedAt - startedAt },
    platform: {
      os: `${process.platform} ${process.arch}`,
      browser: browser.browserType().name(),
      browserVersion: browser.version(),
    },
  });
});
