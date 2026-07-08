import { act, render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import { useShallow } from 'zustand/react/shallow';

import {
  DesktopShellStoreContext,
  createDesktopShellStore,
  useDesktopShellStore,
} from '@/shell/store';

// 購読の分離 smoke(WP-H6 PR2)。
// 3 秒ポーリングが更新するフィールド(syncStatus / communityNodeStatuses)を
// setState しても、それを選択していない購読者が再レンダーされないことを固定する。
// selector なしの全ストア購読(useDesktopShellStore())ではこの分離が成立しない。

let unrelatedRenders = 0;
let relatedRenders = 0;

function UnrelatedProbe() {
  const { composer } = useDesktopShellStore(useShallow((s) => ({ composer: s.composer })));
  unrelatedRenders += 1;
  return <div data-testid="unrelated">{composer}</div>;
}

function RelatedProbe() {
  const localAuthorPubkey = useDesktopShellStore((s) => s.syncStatus.local_author_pubkey);
  relatedRenders += 1;
  return <div data-testid="related">{localAuthorPubkey}</div>;
}

describe('shell 購読の分離(レンダー回数 smoke)', () => {
  it('ポーリング更新フィールドを選択しない購読者は再レンダーされない', () => {
    const store = createDesktopShellStore();
    unrelatedRenders = 0;
    relatedRenders = 0;
    render(
      <DesktopShellStoreContext.Provider value={store}>
        <UnrelatedProbe />
        <RelatedProbe />
      </DesktopShellStoreContext.Provider>
    );
    expect(unrelatedRenders).toBe(1);
    expect(relatedRenders).toBe(1);

    // ポーリング 1 周期相当: syncStatus を新しいオブジェクトで更新する
    act(() => {
      const current = store.getState().syncStatus;
      store.setState({
        syncStatus: { ...current, last_synced_at: (current.last_synced_at ?? 0) + 1 },
      });
    });

    // 無関係フィールドの購読者は再レンダーされない(選択結果が同一のため)
    expect(unrelatedRenders).toBe(1);
    // syncStatus 由来のスカラーを選択する購読者は、値が変わらなければ再レンダーされない
    expect(relatedRenders).toBe(1);

    // 選択しているスカラー自体が変わったときだけ再レンダーされる
    act(() => {
      const current = store.getState().syncStatus;
      store.setState({
        syncStatus: { ...current, local_author_pubkey: 'changed-pubkey' },
      });
    });
    expect(unrelatedRenders).toBe(1);
    expect(relatedRenders).toBe(2);
    expect(screen.getByTestId('related').textContent).toBe('changed-pubkey');
  });
});
