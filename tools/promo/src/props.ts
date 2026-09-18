import { assertPromoManifest, type PromoManifest } from './manifest';

/**
 * composition へ渡す props。CLI からは JSON ファイルで渡す
 * (`--props=<path>`)。Windows の shell quoting を避けるため、
 * props を inline の JSON 文字列で渡す運用はしない。
 */
export type SceneProps = {
  manifest: PromoManifest;
  /** 焼き込む字幕。無音で理解できるようにするため、動画では原則必須。 */
  caption: string | null;
  /** デモ表記を出すか。既定は true で、false は実機でない合成画面以外に使わない。 */
  demoBadge: boolean;
  /** 出力の fps。still では使わない。 */
  fps: number;
};

export const DEFAULT_FPS = 30;

/**
 * 欠落 props / 不正 props を render 開始前に失敗させる。
 * Remotion の defaultProps は「編集中の見本」であり、
 * 本番 render で欠落入力を黙って埋めるために使わない。
 */
export function parseSceneProps(input: unknown): SceneProps {
  if (typeof input !== 'object' || input === null) {
    throw new Error('promo props: object ではない');
  }
  const raw = input as Record<string, unknown>;
  if (raw.manifest === undefined) {
    throw new Error('promo props: manifest が無い。--props=<manifest を含む JSON> を渡す');
  }
  const manifest = assertPromoManifest(raw.manifest);

  const caption = raw.caption === undefined || raw.caption === null ? null : String(raw.caption);
  const demoBadge = raw.demoBadge === undefined ? true : Boolean(raw.demoBadge);
  const fps = typeof raw.fps === 'number' && raw.fps > 0 ? raw.fps : DEFAULT_FPS;

  return { manifest, caption, demoBadge, fps };
}

/** clip の長さ (ms) から frame 数を出す。clip が無い素材は 1 frame 扱い。 */
export function clipDurationInFrames(props: SceneProps): number {
  const clip = props.manifest.clip;
  if (!clip) {
    return 1;
  }
  const durationMs = clip.endMs - clip.startMs;
  if (!(durationMs > 0)) {
    throw new Error(`promo props: clip の長さが 0 以下 (${clip.startMs}..${clip.endMs})`);
  }
  return Math.max(1, Math.round((durationMs / 1000) * props.fps));
}
