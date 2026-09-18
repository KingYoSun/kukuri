/**
 * 原素材 manifest の形。撮影(capture)側が書き、Remotion 側が読む共有契約。
 *
 * この型を変える場合は、書き手 (apps/desktop/tests/promo/promoArtifacts.ts) と
 * 読み手 (tools/promo/src/compositions/*) を同じ差分で更新する。
 */

export type PromoSourceMode = 'mock' | 'device';

export type PromoManifest = {
  /** 台本の場面ID。docs/progress/2026-09-15-promo-lp-brief.md の shot list と対応する。 */
  sceneId: string;
  /** cut ID。1 つの scene に複数 cut がある場合に区別する。 */
  cutId: string;
  /** 撮影対象の commit。配布候補 release の commit、または mock 撮影時の作業 commit。 */
  sourceCommit: string;
  /** 撮影対象の release tag。mock 撮影で release に紐づかない場合は null。 */
  sourceRelease: string | null;
  /** mock = browser mock 撮影、device = 実機撮影。 */
  sourceMode: PromoSourceMode;
  /** 画面の表示言語。 */
  locale: 'ja' | 'en';
  /** アプリのテーマ。 */
  theme: 'dark' | 'light';
  /** 開発者モードの状態。実験機能を含む素材は true になる。 */
  developerMode: boolean;
  /** 撮影時の viewport。録画解像度と一致させる。 */
  viewport: { width: number; height: number };
  /** 録画解像度。viewport と異なる場合は縮小が起きているので失敗として扱う。 */
  recordSize: { width: number; height: number };
  /** 撮影した OS と browser。 */
  platform: { os: string; browser: string; browserVersion: string };
  /** 画面に焼き込まれるフォント。未指定ならシステムフォント依存であることを示す。 */
  fonts: string[];
  /** 採用する区間 (ms)。動画がない静止画のみの素材では null。 */
  clip: { startMs: number; endMs: number } | null;
  /** 撮影時刻 (ISO 8601)。 */
  capturedAt: string;
  /** publicDir からの相対パス。存在しない出力は null。 */
  files: { video: string | null; still: string | null };
  /** files と同じ key の SHA-256。 */
  checksums: { video: string | null; still: string | null };
  /** 画像に焼き込まず、画像の外（LP の本文など）に置く説明文。 */
  externalCaption?: string | null;
  /** 取り込んだ素材の由来の補足（撮影者からの許可や、画面の読み方の注意など）。 */
  notes?: string[];
};

export const PROMO_MANIFEST_VERSION = 1;

export type PromoManifestFile = {
  version: typeof PROMO_MANIFEST_VERSION;
  manifest: PromoManifest;
};

/**
 * 欠落・不正な入力を render の前に失敗させる。成功扱いの空出力を作らないための guard。
 */
export function assertPromoManifest(value: unknown): PromoManifest {
  if (typeof value !== 'object' || value === null) {
    throw new Error('promo manifest: object ではない');
  }
  const m = value as Record<string, unknown>;
  const required = [
    'sceneId',
    'cutId',
    'sourceCommit',
    'sourceMode',
    'locale',
    'theme',
    'developerMode',
    'viewport',
    'recordSize',
    'platform',
    'capturedAt',
    'files',
  ];
  for (const key of required) {
    if (m[key] === undefined) {
      throw new Error(`promo manifest: ${key} が無い`);
    }
  }
  if (m.sourceMode !== 'mock' && m.sourceMode !== 'device') {
    throw new Error(`promo manifest: sourceMode が不正 (${String(m.sourceMode)})`);
  }
  if (m.locale !== 'ja' && m.locale !== 'en') {
    throw new Error(`promo manifest: locale が不正 (${String(m.locale)})`);
  }
  const viewport = m.viewport as { width?: unknown; height?: unknown };
  const recordSize = m.recordSize as { width?: unknown; height?: unknown };
  if (typeof viewport.width !== 'number' || typeof viewport.height !== 'number') {
    throw new Error('promo manifest: viewport が不正');
  }
  if (typeof recordSize.width !== 'number' || typeof recordSize.height !== 'number') {
    throw new Error('promo manifest: recordSize が不正');
  }
  // 縮小された録画を後段で気付けないまま採用しない。
  if (viewport.width !== recordSize.width || viewport.height !== recordSize.height) {
    throw new Error(
      `promo manifest: 録画解像度が viewport と一致しない ` +
        `(viewport ${viewport.width}x${viewport.height} / record ${recordSize.width}x${recordSize.height})`
    );
  }
  const files = m.files as { video?: unknown; still?: unknown };
  if (files.video == null && files.still == null) {
    throw new Error('promo manifest: video と still がどちらも無い');
  }
  return value as PromoManifest;
}
