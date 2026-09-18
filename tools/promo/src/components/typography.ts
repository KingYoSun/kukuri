/**
 * 媒体別の静止画で共通に使う色と文字の大きさ (#1041)。
 * LP と同じく、明るいニュートラル＋オレンジ。コントラストは LP と同じ組み合わせで 4.5:1 以上。
 */

export const theme = {
  bg: '#f6f4f1',
  text: '#1c1a17',
  textMuted: '#4a4540',
  textSoft: '#6b645c',
  accent: '#d77d45',
  accentStrong: '#9c4f20',
  onAccent: '#20160e',
  fontFamily:
    "system-ui, -apple-system, 'Segoe UI', 'Hiragino Sans', 'Yu Gothic UI', 'Noto Sans JP', sans-serif",
};

/**
 * 画像の幅に比例した文字の大きさ。スマートフォンで縮小表示されても見出しが読めるよう、
 * 見出しは画像幅の約 4.4% を下限にする（1270px 幅で 56px。375px 幅に縮小して約 17px）。
 */
export function typeScale(width: number, layout: 'split' | 'header' | 'icon') {
  const unit = width / 1270;
  const headline = layout === 'header' ? 50 : 56;
  return {
    brand: Math.round(30 * unit),
    eyebrow: Math.round(24 * unit),
    headline: Math.round(headline * unit),
    subhead: Math.round(25 * unit),
    footer: Math.round(19 * unit),
  };
}
