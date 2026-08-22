import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

// Shell stylesheets that ship together (see index.css). Every class token they
// declare must appear somewhere in the source tree, otherwise the rule is dead
// weight that manual sweeps keep missing (WP-H8 left 4 behind, #520 orphaned 4
// more — this gate replaces that manual inventory; WP-B12).
const CSS_FILES = [
  'tokens.css',
  'base.css',
  'shell-phase1-part1.css',
  'shell-phase1-part2.css',
  'shell-phase1-part3.css',
  'shell-phase1-part4.css',
  'shell-scoped-overrides.css',
  'column-span-workspace.css',
] as const;

// Classes that are intentionally declared without a source reference (injected
// by third-party libraries at runtime, kept for documented compatibility, …).
// Keep this list short and justified — an entry here is a documented decision,
// not a way to silence the gate.
const ALLOWED_UNUSED: ReadonlySet<string> = new Set([]);

// Vitest runs with apps/desktop as the working directory (see package.json).
const APP_DIR = process.cwd();
const STYLES_DIR = resolve(APP_DIR, 'src/styles');
const SOURCE_ROOTS = [resolve(APP_DIR, 'src'), resolve(APP_DIR, 'index.html')];
const SOURCE_EXTENSIONS = ['.ts', '.tsx', '.html'];

/** Class tokens declared in selector position (`.foo`), pseudo-classes,
 * attribute selectors and element selectors excluded. */
function collectDeclaredClasses(css: string): Set<string> {
  const noComments = css.replace(/\/\*[\s\S]*?\*\//g, '');
  const classes = new Set<string>();
  // Selector text is whatever sits between the previous `{`/`}` and the next
  // `{`. `@media`/`@supports` wrappers contribute no class tokens themselves
  // and their inner rules are still visited; `@keyframes` frames are numeric.
  for (const match of noComments.matchAll(/(?:^|[{}])([^{}]*)\{/g)) {
    const selector = match[1];
    if (selector.trim().startsWith('@')) {
      continue;
    }
    for (const token of selector.matchAll(/\.([A-Za-z][\w-]*)/g)) {
      classes.add(token[1]);
    }
  }
  return classes;
}

function collectSourceFiles(root: string, files: string[] = []): string[] {
  if (statSync(root).isFile()) {
    files.push(root);
    return files;
  }
  for (const entry of readdirSync(root, { withFileTypes: true })) {
    const path = join(root, entry.name);
    if (entry.isDirectory()) {
      collectSourceFiles(path, files);
    } else if (SOURCE_EXTENSIONS.some((ext) => entry.name.endsWith(ext))) {
      files.push(path);
    }
  }
  return files;
}

/** Every `[\w-]+` word in the source tree. Class names always surface as
 * complete tokens (audited: no `\`foo-${bar}\`` style className fragments in
 * this codebase — conditional concatenation only ever appends whole tokens),
 * so token-set membership is an exact usage check. */
function collectSourceTokens(): Set<string> {
  const tokens = new Set<string>();
  for (const file of SOURCE_ROOTS.flatMap((root) => collectSourceFiles(root))) {
    for (const token of readFileSync(file, 'utf8').split(/[^\w-]+/)) {
      if (token) {
        tokens.add(token);
      }
    }
  }
  return tokens;
}

describe('shell CSS selector usage', () => {
  const declared = new Set<string>(
    CSS_FILES.flatMap((name) => [
      ...collectDeclaredClasses(readFileSync(resolve(STYLES_DIR, name), 'utf8')),
    ]),
  );
  const used = collectSourceTokens();

  it('references every declared class token from the source tree', () => {
    const unused = [...declared]
      .filter((name) => !used.has(name))
      .filter((name) => !ALLOWED_UNUSED.has(name))
      .sort();
    expect(unused).toEqual([]);
  });

  it('keeps the allowlist free of entries that are referenced again', () => {
    const stale = [...ALLOWED_UNUSED].filter((name) => used.has(name)).sort();
    expect(stale).toEqual([]);
  });
});
