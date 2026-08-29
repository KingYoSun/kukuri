import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

const REPOSITORY_ROOT = resolve(process.cwd(), '../..');
const DESIGN_PATH = resolve(REPOSITORY_ROOT, 'DESIGN.md');
const TOKENS_PATH = resolve(process.cwd(), 'src/styles/tokens.css');
const CONTRACT_START = '<!-- TOKEN_CONTRACT_START -->';
const CONTRACT_END = '<!-- TOKEN_CONTRACT_END -->';

type Scope = 'global' | 'dark' | 'light';

type TokenEntry = {
  scope: Scope;
  name: string;
  value: string;
};

function normalizeValue(value: string) {
  return value.trim().replace(/\s+/g, ' ');
}

function parseDeclarations(css: string) {
  return [...css.matchAll(/(--[\w-]+)\s*:\s*([^;]+);/g)].map((match) => ({
    name: match[1],
    value: normalizeValue(match[2]),
  }));
}

function selectorBlock(css: string, selector: RegExp) {
  const match = css.match(new RegExp(`${selector.source}\\s*\\{([\\s\\S]*?)\\n\\}`, 'm'));
  if (!match) throw new Error(`tokens.css selector not found: ${selector.source}`);
  return match[1];
}

function runtimeEntries(css: string): TokenEntry[] {
  const globalBlock = css.slice(0, css.indexOf('@media'));
  const darkBlock = selectorBlock(css, /:root,\s*:root\[data-theme='dark'\]/);
  const lightBlock = selectorBlock(css, /:root\[data-theme='light'\]/);
  return ([
    ['global', globalBlock],
    ['dark', darkBlock],
    ['light', lightBlock],
  ] as const).flatMap(([scope, block]) =>
    parseDeclarations(block).map(({ name, value }) => ({ scope, name, value }))
  );
}

function documentedEntries(markdown: string): TokenEntry[] {
  const start = markdown.indexOf(CONTRACT_START);
  const end = markdown.indexOf(CONTRACT_END);
  expect(start, 'DESIGN.md must contain TOKEN_CONTRACT_START').toBeGreaterThanOrEqual(0);
  expect(end, 'DESIGN.md must contain TOKEN_CONTRACT_END').toBeGreaterThan(start);
  const section = markdown.slice(start + CONTRACT_START.length, end);
  return section.split(/\r?\n/).flatMap((line) => {
    const match = line.match(
      /^\|\s*(global|dark|light)\s*\|\s*`(--[\w-]+)`\s*\|\s*`(.+)`\s*\|$/
    );
    if (!match) return [];
    return [{ scope: match[1] as Scope, name: match[2], value: normalizeValue(match[3]) }];
  });
}

function sorted(entries: TokenEntry[]) {
  return [...entries].sort((left, right) =>
    `${left.scope}:${left.name}`.localeCompare(`${right.scope}:${right.name}`)
  );
}

describe('DESIGN.md token contract', () => {
  it('matches every runtime token name and value in tokens.css', () => {
    const design = readFileSync(DESIGN_PATH, 'utf8');
    const tokens = readFileSync(TOKENS_PATH, 'utf8');
    expect(sorted(documentedEntries(design))).toEqual(sorted(runtimeEntries(tokens)));
  });
});
