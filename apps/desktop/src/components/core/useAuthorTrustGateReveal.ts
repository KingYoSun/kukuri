import { useCallback, useState } from 'react';

import type { AuthorTrustGate } from '@/lib/api';
import type { AuthorTrustGateView } from '@/shell/authorTrustGates';

/// #1061: live / game 一覧で、主催者の表示判断と「表示する」を扱う。
///
/// 「表示する」はその一覧の表示だけに効き、判断も端末の設定も変えない（ADR 0026 §8.4）。
export function useAuthorTrustGateReveal(gates: Record<string, AuthorTrustGate> | undefined) {
  const [revealed, setRevealed] = useState<readonly string[]>([]);
  const gateFor = useCallback(
    (hostPubkey: string | null | undefined): AuthorTrustGateView | null => {
      const pubkey = hostPubkey?.trim();
      if (!pubkey || revealed.includes(pubkey)) return null;
      const gate = gates?.[pubkey];
      if (!gate?.hidden) return null;
      return {
        authorPubkey: pubkey,
        nodeBaseUrl: gate.node_base_url ?? null,
        reasons: gate.reasons,
        fromRepostSource: false,
      };
    },
    [gates, revealed]
  );
  const reveal = useCallback((pubkey: string) => {
    setRevealed((current) => (current.includes(pubkey) ? current : [...current, pubkey]));
  }, []);
  return { gateFor, reveal };
}
