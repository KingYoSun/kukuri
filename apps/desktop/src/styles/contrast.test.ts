import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

// WCAG 2.2 AA gate for the light theme semantic pairs enumerated in Issue #828.
// The dark theme is out of scope here (audited separately in Issue #824).
const TOKENS_PATH = resolve(process.cwd(), 'src/styles/tokens.css');

const NORMAL_TEXT = 4.5;
const NON_TEXT = 3;

type Pair = {
  background: string;
  foreground: string;
  minimum: number;
  usage: string;
};

// Real usage pairs: bg/fg token names as rendered together in the app.
const LIGHT_PAIRS: Pair[] = [
  { background: '--surface-button-primary', foreground: '--primary-foreground', minimum: NORMAL_TEXT, usage: '.button primary / .shell-skip-link' },
  { background: '--surface-button-primary-hover', foreground: '--primary-foreground', minimum: NORMAL_TEXT, usage: '.button primary hover' },
  { background: '--surface-destructive-soft', foreground: '--destructive', minimum: NORMAL_TEXT, usage: 'notice/badge destructive, SettingsMetricGrid' },
  { background: '--surface-panel', foreground: '--destructive', minimum: NORMAL_TEXT, usage: 'field danger hint, .error on panels' },
  { background: '--background', foreground: '--destructive', minimum: NORMAL_TEXT, usage: 'error text on shell background' },
  { background: '--surface-panel-accent', foreground: '--destructive', minimum: NORMAL_TEXT, usage: 'error text on accent panels' },
  { background: '--surface-panel', foreground: '--danger', minimum: NORMAL_TEXT, usage: 'danger text on panels' },
  { background: '--surface-accent-soft', foreground: '--accent-foreground', minimum: NORMAL_TEXT, usage: 'notice/badge accent' },
  { background: '--surface-warning-soft', foreground: '--foreground', minimum: NORMAL_TEXT, usage: 'notice/badge warning' },
  { background: '--surface-warning-soft', foreground: '--warning', minimum: NORMAL_TEXT, usage: 'warning text inside warning surfaces' },
  { background: '--surface-panel', foreground: '--warning', minimum: NORMAL_TEXT, usage: 'warning status text on panels' },
  { background: '--background', foreground: '--warning', minimum: NORMAL_TEXT, usage: 'warning status text on shell background' },
  { background: '--surface-badge-neutral', foreground: '--muted-foreground', minimum: NORMAL_TEXT, usage: 'neutral badge' },
  { background: '--surface-panel-muted', foreground: '--foreground', minimum: NORMAL_TEXT, usage: 'neutral notice, ghost button' },
  { background: '--surface-input', foreground: '--foreground', minimum: NORMAL_TEXT, usage: 'inputs' },
  { background: '--surface-input', foreground: '--muted-foreground-soft', minimum: NORMAL_TEXT, usage: 'input placeholder' },
  { background: '--surface-button-secondary', foreground: '--foreground', minimum: NORMAL_TEXT, usage: 'secondary button' },
  { background: '--background', foreground: '--foreground', minimum: NORMAL_TEXT, usage: 'body text on shell background' },
  { background: '--surface-panel', foreground: '--foreground', minimum: NORMAL_TEXT, usage: 'body text on panels' },
  { background: '--background', foreground: '--muted-foreground', minimum: NORMAL_TEXT, usage: 'muted text on shell background' },
  { background: '--surface-panel', foreground: '--muted-foreground', minimum: NORMAL_TEXT, usage: 'muted text on panels' },
  { background: '--surface-panel-accent', foreground: '--muted-foreground', minimum: NORMAL_TEXT, usage: 'muted text on accent panels' },
  { background: '--background', foreground: '--muted-foreground-soft', minimum: NORMAL_TEXT, usage: 'soft muted text on shell background' },
  { background: '--surface-panel', foreground: '--muted-foreground-soft', minimum: NORMAL_TEXT, usage: 'soft muted text on panels' },
  { background: '--surface-panel-accent', foreground: '--muted-foreground-soft', minimum: NORMAL_TEXT, usage: 'soft muted text on accent panels' },
  { background: '--surface-panel', foreground: '--accent', minimum: NORMAL_TEXT, usage: 'accent links/stat text on panels' },
  { background: '--background', foreground: '--accent', minimum: NORMAL_TEXT, usage: 'accent text on shell background' },
  { background: '--surface-selection', foreground: '--foreground', minimum: NORMAL_TEXT, usage: '::selection' },
  { background: '--surface-destructive-soft', foreground: '--border-destructive', minimum: NON_TEXT, usage: 'destructive notice/badge border' },
  { background: '--surface-panel', foreground: '--border-destructive', minimum: NON_TEXT, usage: 'destructive border on panels' },
  { background: '--surface-warning-soft', foreground: '--border-warning', minimum: NON_TEXT, usage: 'warning notice/badge border' },
  { background: '--surface-panel', foreground: '--border-warning', minimum: NON_TEXT, usage: 'warning border on panels' },
  { background: '--surface-accent-soft', foreground: '--border-accent', minimum: NON_TEXT, usage: 'accent border on accent surfaces' },
  { background: '--surface-panel', foreground: '--border-accent', minimum: NON_TEXT, usage: 'accent selected-state border on panels' },
  { background: '--surface-panel-soft', foreground: '--border-accent', minimum: NON_TEXT, usage: 'accent border on soft panels' },
  { background: '--surface-panel', foreground: '--surface-button-primary', minimum: NON_TEXT, usage: 'primary button boundary against panels' },
];

// Focus ring is semi-transparent: composite over each backdrop before comparing.
const RING_BACKDROPS = ['--surface-panel', '--background', '--surface-input'];

type Rgb = readonly [number, number, number];

function lightTokens(css: string) {
  const match = css.match(/:root\[data-theme='light'\]\s*\{([\s\S]*?)\n\}/);
  if (!match) throw new Error("tokens.css light block not found");
  const entries = [...match[1].matchAll(/(--[\w-]+)\s*:\s*([^;]+);/g)].map(
    (declaration) => [declaration[1], declaration[2].trim()] as const
  );
  return new Map(entries);
}

function parseHex(value: string): Rgb {
  const match = value.match(/^#([0-9a-f]{6})$/i);
  if (!match) throw new Error(`expected 6-digit hex color, got: ${value}`);
  const channels = match[1];
  return [0, 2, 4].map((offset) => parseInt(channels.slice(offset, offset + 2), 16)) as unknown as Rgb;
}

function parseRgba(value: string): { rgb: Rgb; alpha: number } {
  const match = value.match(/^rgba\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*,\s*([\d.]+)\s*\)$/);
  if (!match) throw new Error(`expected rgba() color, got: ${value}`);
  return { rgb: [Number(match[1]), Number(match[2]), Number(match[3])], alpha: Number(match[4]) };
}

function compositeOver(foreground: Rgb, alpha: number, backdrop: Rgb): Rgb {
  return foreground.map(
    (channel, index) => alpha * channel + (1 - alpha) * backdrop[index]
  ) as unknown as Rgb;
}

function relativeLuminance([r, g, b]: Rgb) {
  const [lr, lg, lb] = [r, g, b].map((channel) => {
    const c = channel / 255;
    return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * lr + 0.7152 * lg + 0.0722 * lb;
}

function contrastRatio(a: Rgb, b: Rgb) {
  const [lighter, darker] = [relativeLuminance(a), relativeLuminance(b)].sort(
    (left, right) => right - left
  );
  return (lighter + 0.05) / (darker + 0.05);
}

describe('light theme WCAG 2.2 AA contrast (Issue #828)', () => {
  const tokens = lightTokens(readFileSync(TOKENS_PATH, 'utf8'));

  function tokenValue(name: string) {
    const value = tokens.get(name);
    if (!value) throw new Error(`light token not declared: ${name}`);
    return value;
  }

  it.each(LIGHT_PAIRS)(
    '$foreground on $background is at least $minimum:1 ($usage)',
    ({ background, foreground, minimum }) => {
      const ratio = contrastRatio(parseHex(tokenValue(background)), parseHex(tokenValue(foreground)));
      expect(ratio).toBeGreaterThanOrEqual(minimum);
    }
  );

  it.each(RING_BACKDROPS)('--ring composited over %s is at least 3:1', (backdropToken) => {
    const backdrop = parseHex(tokenValue(backdropToken));
    const ring = parseRgba(tokenValue('--ring'));
    const ratio = contrastRatio(compositeOver(ring.rgb, ring.alpha, backdrop), backdrop);
    expect(ratio).toBeGreaterThanOrEqual(NON_TEXT);
  });
});
