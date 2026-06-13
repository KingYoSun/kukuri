import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

// Shell stylesheets that ship together (see index.css). Any `var(--token)` they
// reference must resolve to a custom property defined in the same bundle,
// otherwise the value silently falls back to nothing at runtime.
const CSS_FILES = ['tokens.css', 'base.css', 'shell-phase1.css', 'shell-phase1-legacy.css'] as const;

// Custom properties intentionally injected at runtime (inline style / JS) rather
// than declared in the static stylesheets. Tailwind owns the `--tw-*` space.
const RUNTIME_VARS = new Set(['--shell-detail-pane-index']);
const RUNTIME_PREFIXES = ['--tw-'];

// Vitest runs with apps/desktop as the working directory (see package.json).
const STYLES_DIR = resolve(process.cwd(), 'src/styles');

function readCss(name: string): string {
  return readFileSync(resolve(STYLES_DIR, name), 'utf8');
}

function collectDefinitions(css: string): Set<string> {
  const defined = new Set<string>();
  const declaration = /(--[\w-]+)\s*:/g;
  let match: RegExpExecArray | null;
  while ((match = declaration.exec(css)) !== null) {
    defined.add(match[1]);
  }
  return defined;
}

function collectReferences(css: string): Set<string> {
  const referenced = new Set<string>();
  const usage = /var\(\s*(--[\w-]+)/g;
  let match: RegExpExecArray | null;
  while ((match = usage.exec(css)) !== null) {
    referenced.add(match[1]);
  }
  return referenced;
}

describe('shell CSS custom properties', () => {
  const bundle = CSS_FILES.map(readCss).join('\n');
  const defined = collectDefinitions(bundle);
  const referenced = collectReferences(bundle);

  it('defines every custom property referenced via var()', () => {
    const undefinedRefs = [...referenced]
      .filter((name) => !defined.has(name))
      .filter((name) => !RUNTIME_VARS.has(name))
      .filter((name) => !RUNTIME_PREFIXES.some((prefix) => name.startsWith(prefix)))
      .sort();
    expect(undefinedRefs).toEqual([]);
  });
});
