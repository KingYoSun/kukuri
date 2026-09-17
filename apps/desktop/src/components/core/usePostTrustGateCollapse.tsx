import { useState } from 'react';

import type { AuthorTrustGateView } from '@/shell/authorTrustGates';

import { AuthorTrustGateNotice } from './AuthorTrustGateNotice';

/// 投稿カードを折りたたむ表示を返す（`PostCard` から使う）。
///
/// 「表示する」はこの投稿だけに効き、判断も端末の設定も変えない。判断が無い、または既に
/// 表示した場合は `null` を返し、カードを通常どおり描画させる。
export function usePostTrustGateCollapse(
  gate: AuthorTrustGateView | null | undefined,
  onOpenAuthor?: (authorPubkey: string) => void
) {
  const [revealed, setRevealed] = useState(false);
  if (!gate || revealed) return null;
  return (
    <div className='post-layout-safe'>
      <AuthorTrustGateNotice
        gate={gate}
        onReveal={() => setRevealed(true)}
        onOpenAuthor={onOpenAuthor}
      />
    </div>
  );
}

