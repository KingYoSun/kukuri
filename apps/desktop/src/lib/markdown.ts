// #1106: Community Node のポリシー本文(cn-operator が生成する Markdown と運営者の補足)を
// 構造化する小さな parser。HTML 文字列は生成せず、描画側は React 要素だけを組み立てる。
// raw HTML は解釈しないため、`<script>` 等は通常の文字列として残る。

export { safeExternalHref } from './externalUrls';

export type MarkdownInline =
  | { type: 'text'; value: string }
  | { type: 'strong'; children: MarkdownInline[] }
  | { type: 'emphasis'; children: MarkdownInline[] }
  | { type: 'code'; value: string }
  | { type: 'link'; href: string; children: MarkdownInline[] }
  | { type: 'break' };

export type MarkdownBlock =
  | { type: 'heading'; level: number; children: MarkdownInline[] }
  | { type: 'paragraph'; children: MarkdownInline[] }
  | { type: 'list'; ordered: boolean; start: number; items: MarkdownBlock[][] }
  | { type: 'blockquote'; children: MarkdownBlock[] }
  | { type: 'code'; value: string }
  | { type: 'table'; header: MarkdownInline[][]; rows: MarkdownInline[][][] }
  | { type: 'rule' };

const FENCE = /^ {0,3}(`{3,}|~{3,})/;
const HEADING = /^ {0,3}(#{1,6})(?:[ \t]+(.*?))?(?:[ \t]+#+)?[ \t]*$/;
const RULE = /^ {0,3}([-*_])(?:[ \t]*\1){2,}[ \t]*$/;
const QUOTE = /^ {0,3}> ?/;
const LIST_ITEM = /^( {0,3})([-*+]|\d{1,9}[.)])([ \t]+|$)/;
const TABLE_DELIMITER = /^ {0,3}\|?[ \t]*:?-+:?[ \t]*(?:\|[ \t]*:?-+:?[ \t]*)*\|?[ \t]*$/;
const MAX_DEPTH = 8;

const isBlank = (line: string) => line.trim() === '';
const indentOf = (line: string) => line.length - line.trimStart().length;

function startsTable(lines: string[], index: number) {
  const delimiter = lines[index + 1];
  return lines[index].includes('|') && delimiter !== undefined && delimiter.includes('|') && TABLE_DELIMITER.test(delimiter);
}
// 段落を中断する block の開始行か。
function interruptsParagraph(lines: string[], index: number) {
  const line = lines[index];
  return FENCE.test(line) || HEADING.test(line) || RULE.test(line) || QUOTE.test(line)
    || LIST_ITEM.test(line) || startsTable(lines, index);
}

export function parseMarkdown(source: string): MarkdownBlock[] {
  return parseBlocks(source.replace(/\r\n?/g, '\n').split('\n'), 0);
}

function parseBlocks(lines: string[], depth: number): MarkdownBlock[] {
  const blocks: MarkdownBlock[] = [];
  let index = 0;
  while (index < lines.length) {
    const line = lines[index];
    if (isBlank(line)) {
      index += 1;
      continue;
    }

    const fence = FENCE.exec(line);
    if (fence) {
      const marker = fence[1];
      const body: string[] = [];
      index += 1;
      while (index < lines.length && !new RegExp(`^ {0,3}${marker[0]}{${marker.length},}[ \\t]*$`).test(lines[index])) {
        body.push(lines[index]);
        index += 1;
      }
      index += 1;
      blocks.push({ type: 'code', value: body.join('\n') });
      continue;
    }

    const heading = HEADING.exec(line);
    if (heading) {
      blocks.push({ type: 'heading', level: heading[1].length, children: parseInline(heading[2] ?? '') });
      index += 1;
      continue;
    }

    if (RULE.test(line)) {
      blocks.push({ type: 'rule' });
      index += 1;
      continue;
    }

    if (QUOTE.test(line)) {
      const quoted: string[] = [];
      while (index < lines.length && QUOTE.test(lines[index])) {
        quoted.push(lines[index].replace(QUOTE, ''));
        index += 1;
      }
      blocks.push(depth < MAX_DEPTH
        ? { type: 'blockquote', children: parseBlocks(quoted, depth + 1) }
        : { type: 'paragraph', children: parseInline(quoted.join('\n')) });
      continue;
    }

    const item = LIST_ITEM.exec(line);
    if (item) {
      index = parseList(lines, index, depth, blocks);
      continue;
    }

    if (startsTable(lines, index)) {
      const header = splitRow(line);
      const rows: MarkdownInline[][][] = [];
      index += 2;
      while (index < lines.length && !isBlank(lines[index]) && lines[index].includes('|')) {
        const cells = splitRow(lines[index]);
        rows.push(header.map((_, column) => cells[column] ?? []));
        index += 1;
      }
      blocks.push({ type: 'table', header, rows });
      continue;
    }

    const paragraph = [line.trim()];
    index += 1;
    while (index < lines.length && !isBlank(lines[index]) && !interruptsParagraph(lines, index)) {
      paragraph.push(lines[index].trim());
      index += 1;
    }
    blocks.push({ type: 'paragraph', children: parseInline(paragraph.join('\n')) });
  }
  return blocks;
}

function parseList(lines: string[], start: number, depth: number, blocks: MarkdownBlock[]) {
  const first = LIST_ITEM.exec(lines[start])!;
  const ordered = /\d/.test(first[2]);
  const baseIndent = first[1].length;
  const items: string[][] = [];
  let index = start;
  while (index < lines.length) {
    const line = lines[index];
    const item = LIST_ITEM.exec(line);
    if (!item || item[1].length !== baseIndent || /\d/.test(item[2]) !== ordered) break;
    const contentIndent = item[0].length === line.length ? baseIndent + item[2].length + 1 : item[0].length;
    const content = [line.slice(item[0].length)];
    index += 1;
    while (index < lines.length) {
      const next = lines[index];
      if (isBlank(next)) {
        // 空行の後も、字下げされた続きがあれば同じ item に含める。
        let lookahead = index + 1;
        while (lookahead < lines.length && isBlank(lines[lookahead])) lookahead += 1;
        if (lookahead < lines.length && indentOf(lines[lookahead]) >= contentIndent) {
          content.push('');
          index += 1;
          continue;
        }
        break;
      }
      if (indentOf(next) >= contentIndent) {
        content.push(next.slice(contentIndent));
        index += 1;
        continue;
      }
      // 字下げの無い継続行（lazy continuation）は、新しい block でない場合だけ取り込む。
      if (!interruptsParagraph(lines, index) && !isBlank(content[content.length - 1])) {
        content.push(next.trim());
        index += 1;
        continue;
      }
      break;
    }
    items.push(content);
    // 項目間の空行は同じ list として続ける。
    let lookahead = index;
    while (lookahead < lines.length && isBlank(lines[lookahead])) lookahead += 1;
    const following = lookahead < lines.length ? LIST_ITEM.exec(lines[lookahead]) : null;
    if (following && following[1].length === baseIndent && /\d/.test(following[2]) === ordered) {
      index = lookahead;
    }
  }
  blocks.push({
    type: 'list',
    ordered,
    start: ordered ? Number.parseInt(first[2], 10) : 1,
    items: items.map((content) => depth < MAX_DEPTH
      ? parseBlocks(content, depth + 1)
      : [{ type: 'paragraph', children: parseInline(content.join('\n')) }]),
  });
  return index;
}

function splitRow(line: string): MarkdownInline[][] {
  let row = line.trim();
  if (row.startsWith('|')) row = row.slice(1);
  if (row.endsWith('|') && !row.endsWith('\\|')) row = row.slice(0, -1);
  const cells: string[] = [];
  let current = '';
  for (let index = 0; index < row.length; index += 1) {
    if (row[index] === '\\' && row[index + 1] === '|') {
      current += '|';
      index += 1;
    } else if (row[index] === '|') {
      cells.push(current);
      current = '';
    } else {
      current += row[index];
    }
  }
  cells.push(current);
  return cells.map((cell) => parseInline(cell.trim()));
}

const ESCAPABLE = /[!-/:-@[-`{-~]/;
const BARE_URL = /^https?:\/\/[A-Za-z0-9\-._~:/?#@!$&'*+,;=%]+/;
const URL_TRAILING = /[.,;:!?'*]+$/;
const WORD = /[\p{L}\p{N}]/u;

export function parseInline(source: string, depth = 0): MarkdownInline[] {
  const nodes: MarkdownInline[] = [];
  let text = '';
  const flush = () => {
    if (text) nodes.push({ type: 'text', value: text });
    text = '';
  };
  const push = (node: MarkdownInline) => {
    flush();
    nodes.push(node);
  };

  let index = 0;
  while (index < source.length) {
    const char = source[index];
    const rest = source.slice(index);

    if (char === '\\' && index + 1 < source.length) {
      if (source[index + 1] === '\n') {
        push({ type: 'break' });
        index += 2;
        continue;
      }
      if (ESCAPABLE.test(source[index + 1])) {
        text += source[index + 1];
        index += 2;
        continue;
      }
    }

    if (char === '\n') {
      text = text.replace(/[ \t]+$/, '');
      push({ type: 'break' });
      index += 1;
      while (source[index] === ' ' || source[index] === '\t') index += 1;
      continue;
    }

    if (char === '`') {
      const run = /^`+/.exec(rest)![0];
      const close = source.indexOf(run, index + run.length);
      if (close !== -1) {
        let value = source.slice(index + run.length, close).replace(/\n/g, ' ');
        if (value.length > 2 && value.startsWith(' ') && value.endsWith(' ') && value.trim()) {
          value = value.slice(1, -1);
        }
        push({ type: 'code', value });
        index = close + run.length;
        continue;
      }
      text += run;
      index += run.length;
      continue;
    }

    if (char === '[' && depth < MAX_DEPTH) {
      const link = parseLink(source, index);
      if (link) {
        push({ type: 'link', href: link.href, children: parseInline(link.label, depth + 1) });
        index = link.end;
        continue;
      }
    }

    if (char === '<') {
      const autolink = /^<(https?:\/\/[^\s<>]+)>/.exec(rest);
      if (autolink) {
        push({ type: 'link', href: autolink[1], children: [{ type: 'text', value: autolink[1] }] });
        index += autolink[0].length;
        continue;
      }
    }

    if (char === 'h' && !WORD.test(source[index - 1] ?? '')) {
      const bare = BARE_URL.exec(rest);
      if (bare) {
        const url = bare[0].replace(URL_TRAILING, '');
        if (url.length > 'https://'.length) {
          push({ type: 'link', href: url, children: [{ type: 'text', value: url }] });
          index += url.length;
          continue;
        }
      }
    }

    if ((char === '*' || char === '_') && depth < MAX_DEPTH) {
      const emphasis = parseEmphasis(source, index);
      if (emphasis) {
        push(emphasis.strong
          ? { type: 'strong', children: parseInline(emphasis.inner, depth + 1) }
          : { type: 'emphasis', children: parseInline(emphasis.inner, depth + 1) });
        index = emphasis.end;
        continue;
      }
      const run = char === '*' ? /^\*+/.exec(rest)![0] : /^_+/.exec(rest)![0];
      text += run;
      index += run.length;
      continue;
    }

    text += char;
    index += 1;
  }
  flush();
  return nodes;
}

function parseLink(source: string, start: number) {
  let nesting = 0;
  let labelEnd = -1;
  for (let index = start + 1; index < source.length; index += 1) {
    const char = source[index];
    if (char === '\\') {
      index += 1;
    } else if (char === '`') {
      const close = source.indexOf('`', index + 1);
      if (close !== -1) index = close;
    } else if (char === '[') {
      nesting += 1;
    } else if (char === ']') {
      if (nesting === 0) {
        labelEnd = index;
        break;
      }
      nesting -= 1;
    }
  }
  if (labelEnd === -1 || source[labelEnd + 1] !== '(') return null;
  const destination = /^\([ \t]*(<[^<>\n]*>|[^\s()]+)(?:[ \t]+"[^"\n]*")?[ \t]*\)/.exec(source.slice(labelEnd + 1));
  if (!destination) return null;
  const rawHref = destination[1].startsWith('<') ? destination[1].slice(1, -1) : destination[1];
  return {
    label: source.slice(start + 1, labelEnd),
    href: rawHref,
    end: labelEnd + 1 + destination[0].length,
  };
}

function parseEmphasis(source: string, start: number) {
  const marker = source[start];
  const run = source.slice(start).match(marker === '*' ? /^\*+/ : /^_+/)![0];
  const before = source[start - 1] ?? '';
  const after = source[start + run.length] ?? '';
  if (!after || /\s/.test(after)) return null;
  // `_` は語中(snake_case 等)では強調にしない。
  if (marker === '_' && WORD.test(before)) return null;
  const width = run.length >= 2 ? 2 : 1;
  const delimiter = marker.repeat(width);
  let search = start + width;
  while (search < source.length) {
    const close = source.indexOf(delimiter, search);
    if (close === -1) return null;
    const closeBefore = source[close - 1];
    const closeAfter = source[close + width] ?? '';
    const sameRunContinues = closeAfter === marker && width === 1;
    if (
      close > start + width
      && !/\s/.test(closeBefore)
      && closeBefore !== '\\'
      && !sameRunContinues
      && !(marker === '_' && WORD.test(closeAfter))
    ) {
      return { strong: width === 2, inner: source.slice(start + width, close), end: close + width };
    }
    search = close + (sameRunContinues ? 2 : 1);
  }
  return null;
}
