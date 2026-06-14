import { describe, expect, test } from 'vitest';

import {
  buildMentionToken,
  extractMentions,
  parseSmartText,
} from './internalLinks';

const PUBKEY = 'a'.repeat(64);
const OTHER_PUBKEY = 'b'.repeat(64);

describe('buildMentionToken', () => {
  test('produces an @[label](pubkey) token', () => {
    expect(buildMentionToken('Alice', PUBKEY)).toBe(`@[Alice](${PUBKEY})`);
  });

  test('sanitizes brackets and newlines in the label', () => {
    expect(buildMentionToken('Al]ice\nBob', PUBKEY)).toBe(`@[Al ice Bob](${PUBKEY})`);
  });

  test('falls back to a shortened pubkey when the label is empty', () => {
    const token = buildMentionToken('   ', PUBKEY);
    expect(token).toContain(`(${PUBKEY})`);
    expect(token).not.toBe(`@[](${PUBKEY})`);
  });
});

describe('extractMentions', () => {
  test('returns every mention with its label and pubkey', () => {
    const text = `hi @[Alice](${PUBKEY}) and @[Bob](${OTHER_PUBKEY})`;
    expect(extractMentions(text)).toEqual([
      { label: 'Alice', pubkey: PUBKEY },
      { label: 'Bob', pubkey: OTHER_PUBKEY },
    ]);
  });

  test('ignores tokens with an invalid pubkey length', () => {
    expect(extractMentions('@[Alice](abc)')).toEqual([]);
  });
});

describe('parseSmartText mentions', () => {
  test('emits a mention segment for a valid token', () => {
    const segments = parseSmartText(`hello @[Alice](${PUBKEY})!`);
    expect(segments).toHaveLength(1);
    expect(segments[0]).toEqual([
      { kind: 'text', text: 'hello ' },
      { kind: 'mention', label: 'Alice', pubkey: PUBKEY },
      { kind: 'text', text: '!' },
    ]);
  });

  test('keeps order relative to a topic reference on the same line', () => {
    const segments = parseSmartText(`@[Alice](${PUBKEY}) in kukuri:topic:demo`);
    const line = segments[0];
    expect(line[0]).toEqual({ kind: 'mention', label: 'Alice', pubkey: PUBKEY });
    expect(line.some((segment) => segment.kind === 'reference')).toBe(true);
  });

  test('treats an invalid pubkey length as plain text', () => {
    const segments = parseSmartText('@[Alice](deadbeef)');
    expect(segments[0]).toEqual([{ kind: 'text', text: '@[Alice](deadbeef)' }]);
  });

  test('does not falsely match when the label contains a closing bracket', () => {
    const segments = parseSmartText(`@[Bad]label](${PUBKEY})`);
    expect(segments[0].some((segment) => segment.kind === 'mention')).toBe(false);
  });
});
