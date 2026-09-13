import { useEffect, useRef, useState } from 'react';

import type { ExtendedPanelStatus } from '@/components/extended/types';

/** DESIGN.md 4.2: 一覧の初回取得は表示開始から最低この時間 loading を示す(#994)。 */
export const MIN_LIST_LOADING_MS = 500;

/**
 * 一覧の loading を最低時間保つための表示用 status。
 * 実 status が `loading` から抜けても、loading の表示開始から `minMs` 経つまでは `loading` を返し、
 * 取得がそれより長い場合は完了まで続ける。取得の開始・回数・順序は変えず、表示だけを遅らせる(INVAR-4)。
 * `loading` 以外で mount した場合は即座に実 status を返す(値が既にある panel の再 mount で点滅させない)。
 */
export function useMinimumLoading(
  status: ExtendedPanelStatus,
  minMs: number = MIN_LIST_LOADING_MS
): ExtendedPanelStatus {
  // 開始時刻は effect で記録する(render 中に Date.now() を呼ばない)。
  const loadingStartedAt = useRef<number | null>(null);
  const [held, setHeld] = useState(status === 'loading');

  useEffect(() => {
    if (status === 'loading') {
      if (loadingStartedAt.current === null) loadingStartedAt.current = Date.now();
      setHeld(true);
      return;
    }
    const startedAt = loadingStartedAt.current;
    if (startedAt === null) {
      setHeld(false);
      return;
    }
    const remaining = minMs - (Date.now() - startedAt);
    if (remaining <= 0) {
      loadingStartedAt.current = null;
      setHeld(false);
      return;
    }
    const timer = setTimeout(() => {
      loadingStartedAt.current = null;
      setHeld(false);
    }, remaining);
    return () => clearTimeout(timer);
  }, [status, minMs]);

  return held ? 'loading' : status;
}
