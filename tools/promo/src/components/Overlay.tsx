import type { PromoManifest } from '../manifest';

/**
 * 素材の由来表記と字幕。brief (docs/progress/2026-09-15-promo-lp-brief.md) の
 * 共通ルールを実装する。
 *
 * - デモ表記はすべての画面素材に出す
 * - 実機由来の素材は mock と区別できるようにする
 * - 開発者モード限定の実験機能を含む素材は、その旨を必ず添える
 * - 文字は各辺 8% のセーフエリアの内側に置く
 */

const SAFE_AREA_RATIO = 0.08;

type OverlayProps = {
  manifest: PromoManifest;
  caption: string | null;
  demoBadge: boolean;
  width: number;
  height: number;
};

function badgeLabel(manifest: PromoManifest): string {
  return manifest.locale === 'ja' ? 'デモ画面' : 'Demo screen';
}

function sourceLabel(manifest: PromoManifest): string | null {
  if (manifest.sourceMode !== 'device') {
    return null;
  }
  return manifest.locale === 'ja' ? '実機' : 'Real devices';
}

function experimentalNotice(manifest: PromoManifest): string | null {
  if (!manifest.developerMode) {
    return null;
  }
  return manifest.locale === 'ja'
    ? '実験中の機能です。開発者モードでのみ利用でき、データ形式の互換は保証しません。'
    : 'Experimental. Developer mode only, with no data-format compatibility guarantee.';
}

export function Overlay({ manifest, caption, demoBadge, width, height }: OverlayProps) {
  const inset = Math.round(Math.min(width, height) * SAFE_AREA_RATIO);
  const baseFontSize = Math.round(height * 0.028);
  const notice = experimentalNotice(manifest);
  const source = sourceLabel(manifest);

  return (
    <div
      style={{
        position: 'absolute',
        inset: 0,
        padding: inset,
        display: 'flex',
        flexDirection: 'column',
        justifyContent: 'space-between',
        // system-ui は撮影機の既定フォントに従う。同梱フォントの採用は #1041 で決める。
        fontFamily: 'system-ui, sans-serif',
        pointerEvents: 'none',
      }}
    >
      <div style={{ display: 'flex', gap: Math.round(baseFontSize * 0.5), justifyContent: 'flex-end' }}>
        {source ? <Badge fontSize={baseFontSize} tone='source' text={source} /> : null}
        {demoBadge ? <Badge fontSize={baseFontSize} tone='demo' text={badgeLabel(manifest)} /> : null}
      </div>

      <div style={{ display: 'flex', flexDirection: 'column', gap: Math.round(baseFontSize * 0.5) }}>
        {notice ? (
          <div
            style={{
              alignSelf: 'flex-start',
              backgroundColor: 'rgba(70, 52, 35, 0.92)',
              color: '#ffffff',
              border: '2px solid #bf9358',
              borderRadius: Math.round(baseFontSize * 0.4),
              padding: `${Math.round(baseFontSize * 0.4)}px ${Math.round(baseFontSize * 0.7)}px`,
              fontSize: Math.round(baseFontSize * 0.8),
              lineHeight: 1.4,
              maxWidth: '80%',
            }}
          >
            {notice}
          </div>
        ) : null}
        {caption ? (
          <div
            style={{
              alignSelf: 'flex-start',
              backgroundColor: 'rgba(18, 18, 18, 0.88)',
              color: '#ffffff',
              borderRadius: Math.round(baseFontSize * 0.4),
              padding: `${Math.round(baseFontSize * 0.5)}px ${Math.round(baseFontSize * 0.9)}px`,
              fontSize: baseFontSize,
              lineHeight: 1.4,
              maxWidth: '86%',
            }}
          >
            {caption}
          </div>
        ) : null}
      </div>
    </div>
  );
}

function Badge({
  text,
  fontSize,
  tone,
}: {
  text: string;
  fontSize: number;
  tone: 'demo' | 'source';
}) {
  const demo = tone === 'demo';
  return (
    <div
      style={{
        backgroundColor: demo ? 'rgba(215, 125, 69, 0.95)' : 'rgba(32, 58, 55, 0.95)',
        color: demo ? '#20160e' : '#03dac5',
        border: demo ? 'none' : '2px solid #03dac5',
        borderRadius: Math.round(fontSize * 0.35),
        padding: `${Math.round(fontSize * 0.28)}px ${Math.round(fontSize * 0.6)}px`,
        fontSize: Math.round(fontSize * 0.78),
        fontWeight: 700,
        letterSpacing: '0.02em',
      }}
    >
      {text}
    </div>
  );
}
