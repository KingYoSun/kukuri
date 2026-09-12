import { describe, expect, test } from 'vitest';

import type { DesktopLogEntry, DesktopLogSnapshot } from '@/lib/api';
import {
  DESKTOP_LOGS_EXPORT_FILE_NAME,
  buildDesktopLogsExport,
  formatDesktopLogBytes,
  formatDesktopLogLine,
  lastDesktopLogSeq,
  mergeDesktopLogSnapshot,
} from './desktopLogs';

function entry(seq: number, message = `line ${seq}`): DesktopLogEntry {
  return { seq, timestamp_ms: 1_789_000_000_000 + seq * 1_000, level: 'INFO', target: 'kukuri', message };
}

function snapshot(entries: DesktopLogEntry[], oldestSeq: number | null, nextSeq: number): DesktopLogSnapshot {
  return { entries, oldest_seq: oldestSeq, next_seq: nextSeq, max_entries: 5, max_bytes: 1024 * 1024 };
}

describe('mergeDesktopLogSnapshot', () => {
  test('first snapshot replaces the view and reports whether older lines were already dropped', () => {
    const fresh = mergeDesktopLogSnapshot(null, snapshot([entry(1), entry(2)], 1, 3));
    expect(fresh.entries.map((item) => item.seq)).toEqual([1, 2]);
    expect(fresh.droppedOlder).toBe(false);
    expect(fresh.gapSinceLastRefresh).toBe(false);
    expect(lastDesktopLogSeq(fresh)).toBe(2);

    const wrapped = mergeDesktopLogSnapshot(null, snapshot([entry(7), entry(8)], 7, 9));
    expect(wrapped.droppedOlder).toBe(true);
    expect(wrapped.gapSinceLastRefresh).toBe(false);
  });

  test('contiguous refresh appends new lines and drops lines the backend no longer keeps', () => {
    const previous = mergeDesktopLogSnapshot(null, snapshot([entry(1), entry(2), entry(3)], 1, 4));
    // backend は 3 件保持のまま 5 まで進み、1〜2 を落とした。afterSeq=3 の差分応答。
    const next = mergeDesktopLogSnapshot(previous, snapshot([entry(4), entry(5)], 3, 6));
    expect(next.entries.map((item) => item.seq)).toEqual([3, 4, 5]);
    expect(next.droppedOlder).toBe(true);
    expect(next.gapSinceLastRefresh).toBe(false);
  });

  test('refresh after the buffer wrapped past the last shown line replaces the view and flags the gap', () => {
    const previous = mergeDesktopLogSnapshot(null, snapshot([entry(1), entry(2)], 1, 3));
    const next = mergeDesktopLogSnapshot(previous, snapshot([entry(10), entry(11)], 10, 12));
    expect(next.entries.map((item) => item.seq)).toEqual([10, 11]);
    expect(next.gapSinceLastRefresh).toBe(true);
    expect(next.droppedOlder).toBe(true);
  });

  test('empty buffer after lines existed still reports dropped lines, and an empty refresh keeps the view', () => {
    const emptied = mergeDesktopLogSnapshot(null, snapshot([], null, 40));
    expect(emptied.entries).toEqual([]);
    expect(emptied.droppedOlder).toBe(true);

    const previous = mergeDesktopLogSnapshot(null, snapshot([entry(1)], 1, 2));
    const unchanged = mergeDesktopLogSnapshot(previous, snapshot([], 1, 2));
    expect(unchanged.entries.map((item) => item.seq)).toEqual([1]);
    expect(unchanged.gapSinceLastRefresh).toBe(false);
  });
});

describe('export text', () => {
  test('lists redaction rules and every shown line in a stable format', () => {
    const view = mergeDesktopLogSnapshot(null, snapshot([entry(1, 'peer lost peer="abc"')], 1, 2));
    const text = buildDesktopLogsExport(view, new Date('2026-09-11T06:17:02.000Z'));
    expect(DESKTOP_LOGS_EXPORT_FILE_NAME).toBe('kukuri-logs.txt');
    expect(text).toContain('# kukuri desktop logs');
    expect(text).toContain('exported_at: 2026-09-11T06:17:02.000Z');
    expect(text).toContain('entries: 1');
    expect(text).toContain('buffer_limit: 5 entries / 1 MiB');
    expect(text).toContain('older_lines_dropped: no');
    expect(text).toContain('secret keys, auth tokens, DM bodies, and passphrases are never logged');
    expect(text.trim().endsWith(formatDesktopLogLine(entry(1, 'peer lost peer="abc"')))).toBe(true);
    expect(formatDesktopLogLine(entry(1, 'x'))).toBe(`${new Date(entry(1).timestamp_ms).toISOString()} INFO  kukuri: x`);
  });

  test('formats buffer sizes in whole units only', () => {
    expect(formatDesktopLogBytes(1024 * 1024)).toBe('1 MiB');
    expect(formatDesktopLogBytes(4 * 1024)).toBe('4 KiB');
    expect(formatDesktopLogBytes(1500)).toBe('1500 B');
  });
});
