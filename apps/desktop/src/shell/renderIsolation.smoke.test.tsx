import { act, render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import { useShallow } from 'zustand/react/shallow';

import {
  DesktopShellStoreContext,
  createDesktopShellStore,
  useDesktopShellStore,
} from '@/shell/store';

// 購読の分離 smoke(WP-H6 PR2)。
// runtime event push / fallback が更新するフィールド(syncStatus / communityNodeStatuses)を
// setState しても、それを選択していない購読者が再レンダーされないことを固定する。
// selector なしの全ストア購読(useDesktopShellStore())ではこの分離が成立しない。

// 再代入ではなく push で記録する(React Compiler の再代入禁止 lint に従う)。
const renderLog: string[] = [];
const countRenders = (name: string) => renderLog.filter((entry) => entry === name).length;

function UnrelatedProbe() {
  const { composer } = useDesktopShellStore(useShallow((s) => ({ composer: s.composer })));
  renderLog.push('unrelated');
  return <div data-testid="unrelated">{composer}</div>;
}

function RelatedProbe() {
  const localAuthorPubkey = useDesktopShellStore((s) => s.syncStatus.local_author_pubkey);
  renderLog.push('related');
  return <div data-testid="related">{localAuthorPubkey}</div>;
}

describe('shell 購読の分離(レンダー回数 smoke)', () => {
  it('接続状態の更新フィールドを選択しない購読者は再レンダーされない', () => {
    const store = createDesktopShellStore();
    renderLog.length = 0;
    render(
      <DesktopShellStoreContext.Provider value={store}>
        <UnrelatedProbe />
        <RelatedProbe />
      </DesktopShellStoreContext.Provider>
    );
    expect(countRenders('unrelated')).toBe(1);
    expect(countRenders('related')).toBe(1);

    // runtime event push 相当: syncStatus を新しいオブジェクトで更新する
    act(() => {
      const current = store.getState().syncStatus;
      store.setState({
        syncStatus: { ...current, last_sync_ts: (current.last_sync_ts ?? 0) + 1 },
      });
    });

    // 無関係フィールドの購読者は再レンダーされない(選択結果が同一のため)
    expect(countRenders('unrelated')).toBe(1);
    // syncStatus 由来のスカラーを選択する購読者は、値が変わらなければ再レンダーされない
    expect(countRenders('related')).toBe(1);

    // 選択しているスカラー自体が変わったときだけ再レンダーされる
    act(() => {
      const current = store.getState().syncStatus;
      store.setState({
        syncStatus: { ...current, local_author_pubkey: 'changed-pubkey' },
      });
    });
    expect(countRenders('unrelated')).toBe(1);
    expect(countRenders('related')).toBe(2);
    expect(screen.getByTestId('related').textContent).toBe('changed-pubkey');
  });
});
