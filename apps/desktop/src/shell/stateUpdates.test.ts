import { describe, expect, it } from 'vitest';

import {
  panelError,
  panelLoading,
  panelReady,
  removeRecordEntry,
  setRecordEntry,
  updateRecordEntry,
} from '@/shell/stateUpdates';

describe('stateUpdates', () => {
  it('setRecordEntry は 1 キーだけ差し替えた新オブジェクトを返す', () => {
    const current = { a: 1, b: 2 };
    const next = setRecordEntry('a', 10)(current);
    expect(next).toEqual({ a: 10, b: 2 });
    expect(next).not.toBe(current);
    expect(current).toEqual({ a: 1, b: 2 });
  });

  it('updateRecordEntry は現在値(未定義含む)から導出する', () => {
    const current: Record<string, number> = { a: 1 };
    expect(updateRecordEntry<number>('a', (prev) => (prev ?? 0) + 1)(current)).toEqual({ a: 2 });
    expect(updateRecordEntry<number>('b', (prev) => (prev ?? 0) + 1)(current)).toEqual({
      a: 1,
      b: 1,
    });
  });

  it('removeRecordEntry はキーが無ければ current をそのまま返す(参照同一)', () => {
    const current = { a: 1 };
    expect(removeRecordEntry('missing')(current)).toBe(current);
    const next = removeRecordEntry('a')(current);
    expect(next).toEqual({});
    expect(next).not.toBe(current);
  });

  it('パネル状態コンストラクタは status と error を対で揃える', () => {
    expect(panelLoading()).toEqual({ status: 'loading', error: null });
    expect(panelReady()).toEqual({ status: 'ready', error: null });
    expect(panelError('boom')).toEqual({ status: 'error', error: 'boom' });
  });
});
