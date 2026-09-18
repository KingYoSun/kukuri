const MAX_EXTERNAL_URL_LENGTH = 4_096;
const URL_START = /https?:\/\//gi;
const TRAILING_PUNCTUATION = new Set([
  '.', ',', ';', ':', '!', '?', "'", '"', '*',
  '。', '、', '，', '．', '！', '？', '：', '；', '」', '』',
]);
const CLOSING_PAIRS: ReadonlyArray<readonly [string, string]> = [
  ['(', ')'],
  ['[', ']'],
  ['{', '}'],
];

export type ExternalUrlMatch = {
  index: number;
  length: number;
  href: string;
};

function containsControl(value: string): boolean {
  return [...value].some((char) => {
    const code = char.charCodeAt(0);
    return code < 0x20 || code === 0x7f;
  });
}

export function safeExternalHref(href: string): string | null {
  const value = href.trim();
  if (
    value.length === 0 ||
    value.length > MAX_EXTERNAL_URL_LENGTH ||
    !/^https?:\/\//i.test(value) ||
    /[\s\\]/.test(value) ||
    containsControl(value)
  ) {
    return null;
  }
  try {
    const url = new URL(value);
    if (
      !['http:', 'https:'].includes(url.protocol) ||
      !url.hostname ||
      url.username ||
      url.password
    ) {
      return null;
    }
    return value;
  } catch {
    return null;
  }
}

function trimCandidate(candidate: string): string {
  let value = candidate;
  while (value && TRAILING_PUNCTUATION.has(value.at(-1)!)) {
    value = value.slice(0, -1);
  }
  let changed = true;
  while (changed && value) {
    changed = false;
    for (const [open, close] of CLOSING_PAIRS) {
      if (!value.endsWith(close)) continue;
      const opens = [...value].filter((char) => char === open).length;
      const closes = [...value].filter((char) => char === close).length;
      if (closes > opens) {
        value = value.slice(0, -1);
        while (value && TRAILING_PUNCTUATION.has(value.at(-1)!)) {
          value = value.slice(0, -1);
        }
        changed = true;
      }
    }
  }
  return value;
}

function candidateEnd(value: string, start: number): number {
  let index = start;
  while (index < value.length) {
    const char = value[index];
    if (/\s/.test(char) || char === '<' || char === '>' || char === '`' || char === '\\') {
      break;
    }
    index += 1;
  }
  return index;
}

export function findNextExternalUrl(value: string, offset = 0): ExternalUrlMatch | null {
  URL_START.lastIndex = offset;
  let start: RegExpExecArray | null;
  while ((start = URL_START.exec(value)) !== null) {
    const end = candidateEnd(value, start.index);
    if (value[end] === '\\') {
      URL_START.lastIndex = end + 1;
      continue;
    }
    const candidate = trimCandidate(value.slice(start.index, end));
    const href = safeExternalHref(candidate);
    if (href) {
      return { index: start.index, length: candidate.length, href };
    }
    URL_START.lastIndex = Math.max(end, start.index + start[0].length);
  }
  return null;
}

export function firstExternalUrl(value: string): string | null {
  return findNextExternalUrl(value)?.href ?? null;
}
