import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

const STYLES_DIR = resolve(process.cwd(), 'src/styles');

function readCss(name: string): string {
  return readFileSync(resolve(STYLES_DIR, name), 'utf8');
}

describe('native select theme contract', () => {
  const tokens = readCss('tokens.css');
  const base = readCss('base.css');

  it('keeps vertical space for WebKitGTK native select text at a fixed height', () => {
    const select = readFileSync(resolve(process.cwd(), 'src/components/ui/select.tsx'), 'utf8');
    expect(select).toMatch(/\bh-11\b/);
    expect(select).toMatch(/\bpy-0\b/);
    expect(select).not.toMatch(/\bpy-3\b/);
  });

  it('advertises the active app theme to native controls', () => {
    expect(tokens).toMatch(
      /:root,\s*:root\[data-theme='dark'\]\s*\{[^}]*color-scheme:\s*dark;/s
    );
    expect(tokens).toMatch(
      /:root\[data-theme='light'\]\s*\{[^}]*color-scheme:\s*light;/s
    );
  });

  it('gives closed selects and expanded options explicit semantic colors', () => {
    expect(base).toMatch(
      /select\s*\{[^}]*color:\s*var\(--foreground\);[^}]*background(?:-color)?:\s*var\(--surface-input\);/s
    );
    expect(base).toMatch(
      /select\s+option\s*\{[^}]*color:\s*var\(--foreground\);[^}]*background(?:-color)?:\s*var\(--surface-input\);/s
    );
  });
});
