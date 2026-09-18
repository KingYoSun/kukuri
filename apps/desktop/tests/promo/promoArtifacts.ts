import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { mkdirSync, readFileSync, renameSync, rmSync, statSync, writeFileSync } from 'node:fs';
import path from 'node:path';

/**
 * promo 撮影の出力先と manifest の書き出し。
 *
 * 出力は repository root の `promo-artifacts/` に集約し、既存の test-results や
 * visual baseline とは別の場所に置く。Remotion 側はこのディレクトリを
 * public dir として読む (tools/promo/remotion.config.ts)。
 */

export const PROMO_ARTIFACT_ROOT = path.resolve(import.meta.dirname, '../../../../promo-artifacts');
export const CAPTURE_ROOT = path.join(PROMO_ARTIFACT_ROOT, 'captures');

export type CaptureTarget = {
  sceneId: string;
  cutId: string;
  locale: 'ja' | 'en';
  theme: 'dark' | 'light';
  developerMode: boolean;
  viewport: { width: number; height: number };
};

export type CaptureResult = {
  /** publicDir からの相対パス。manifest に載せる。 */
  videoRelative: string | null;
  stillRelative: string | null;
  clip: { startMs: number; endMs: number } | null;
  platform: { os: string; browser: string; browserVersion: string };
};

export function captureDir(target: CaptureTarget): string {
  return path.join(CAPTURE_ROOT, target.sceneId, target.cutId);
}

export function prepareCaptureDir(target: CaptureTarget): string {
  const dir = captureDir(target);
  // 前回の失敗した出力を新しい撮影として扱わないよう、毎回作り直す。
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
  return dir;
}

function sha256(file: string): string {
  return createHash('sha256').update(readFileSync(file)).digest('hex');
}

function currentCommit(): string {
  const fromEnv = process.env.KUKURI_PROMO_SOURCE_COMMIT;
  if (fromEnv && fromEnv.trim().length > 0) {
    return fromEnv.trim();
  }
  return execFileSync('git', ['rev-parse', 'HEAD'], {
    cwd: path.resolve(import.meta.dirname, '../../../..'),
    encoding: 'utf8',
  }).trim();
}

/**
 * Playwright が録画した webm を確定した名前へ移し、存在と大きさを確認する。
 * 空ファイルや欠落を成功として扱わない。
 */
export function finalizeVideo(rawPath: string, dir: string): string {
  const target = path.join(dir, 'video.webm');
  renameSync(rawPath, target);
  const size = statSync(target).size;
  if (size <= 0) {
    throw new Error(`promo capture: 録画ファイルが空 (${target})`);
  }
  return target;
}

export function writeManifest(target: CaptureTarget, result: CaptureResult): string {
  const dir = captureDir(target);
  const videoAbs = result.videoRelative ? path.join(PROMO_ARTIFACT_ROOT, result.videoRelative) : null;
  const stillAbs = result.stillRelative ? path.join(PROMO_ARTIFACT_ROOT, result.stillRelative) : null;

  for (const file of [videoAbs, stillAbs]) {
    if (file && !statSync(file).isFile()) {
      throw new Error(`promo capture: 出力が file ではない (${file})`);
    }
  }

  const manifest = {
    sceneId: target.sceneId,
    cutId: target.cutId,
    sourceCommit: currentCommit(),
    sourceRelease: process.env.KUKURI_PROMO_SOURCE_RELEASE ?? null,
    sourceMode: 'mock' as const,
    locale: target.locale,
    theme: target.theme,
    developerMode: target.developerMode,
    viewport: target.viewport,
    // 録画解像度は viewport と同値で明示する。既定値のままだと縮小された動画になる。
    recordSize: target.viewport,
    platform: result.platform,
    fonts: [],
    clip: result.clip,
    capturedAt: new Date().toISOString(),
    files: { video: result.videoRelative, still: result.stillRelative },
    checksums: {
      video: videoAbs ? sha256(videoAbs) : null,
      still: stillAbs ? sha256(stillAbs) : null,
    },
  };

  const manifestPath = path.join(dir, 'manifest.json');
  writeFileSync(manifestPath, `${JSON.stringify({ version: 1, manifest }, null, 2)}\n`, 'utf8');

  // Remotion へ渡す props も同じ撮影で書き出す。render 時に手で組み立てない。
  const propsPath = path.join(dir, 'props.json');
  writeFileSync(
    propsPath,
    `${JSON.stringify({ manifest, caption: null, demoBadge: true, fps: 30 }, null, 2)}\n`,
    'utf8'
  );

  return manifestPath;
}

export function toPublicRelative(absolute: string): string {
  return path.relative(PROMO_ARTIFACT_ROOT, absolute).split(path.sep).join('/');
}
