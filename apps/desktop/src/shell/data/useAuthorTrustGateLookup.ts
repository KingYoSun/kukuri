import { useEffect, useMemo, useRef } from 'react';

import type {
  AuthorTrustGate,
  CommunityNodeConfig,
  CommunityNodeNodeStatus,
  DesktopApi,
  PostView,
} from '@/lib/api';
import { useDesktopShellStoreApi } from '@/shell/store';

/// 同じ描画更新でまとめて照会するための待ち時間。
export const AUTHOR_TRUST_GATE_LOOKUP_DEBOUNCE_MS = 300;
/// 1 回の照会で送る著者数の上限（CN 側の一括評価の上限と同じ）。
export const AUTHOR_TRUST_GATE_LOOKUP_BATCH_SIZE = 100;
/// 期限を持たない判断（未評価・照会失敗・照会中）を作り直すまでの待ち時間。
export const AUTHOR_TRUST_GATE_LOOKUP_FALLBACK_TTL_MS = 600_000;
/// 期限切れの判断を捨てて照会し直す間隔。
export const AUTHOR_TRUST_GATE_LOOKUP_SWEEP_MS = 30_000;

/// #1061: 投稿カードから、表示判断の対象になる著者を集める。
///
/// 引用・repost では元投稿の著者も対象にする（ADR 0022 と同じ扱い）。
export function postTrustGateAuthors(post: PostView): string[] {
  const authors = [post.author_pubkey, post.repost_of?.source_author_pubkey];
  return authors
    .map((pubkey) => pubkey?.trim())
    .filter((pubkey): pubkey is string => Boolean(pubkey));
}

/// #1061: 採用順位に選んだ node と、その認証・必須同意の状態。
///
/// 同意取消・再認証・CN 削除で変わるため、これが変わったら判断をすべて作り直す
/// （runtime 側の cache 世代の更新と対になる）。
export function trustGateAdoptionSignature(
  config: CommunityNodeConfig,
  statuses: readonly CommunityNodeNodeStatus[]
): string {
  const statusByUrl = new Map(statuses.map((status) => [status.base_url, status]));
  return (config.trust_node_priority ?? [])
    .map((baseUrl) => {
      const status = statusByUrl.get(baseUrl);
      const authenticated = status?.auth_state.authenticated ? '1' : '0';
      const consented = status?.consent_state?.all_required_accepted ? '1' : '0';
      return `${baseUrl}:${authenticated}${consented}`;
    })
    .join('|');
}

/// 判断を使ってよい期限（epoch ms）。期限が無い・読めない場合は fallback を使う。
function gateExpiry(gate: AuthorTrustGate, fallback: number): number {
  if (!gate.expires_at) return fallback;
  const parsed = Date.parse(gate.expires_at);
  return Number.isNaN(parsed) ? fallback : parsed;
}

export type UseAuthorTrustGateLookupArgs = {
  api: Pick<DesktopApi, 'evaluateAuthorTrustGates'>;
  /// 表示中の投稿（タイムライン・スレッド・プロフィール・ブックマーク）。
  posts: readonly PostView[];
  /// live / game 一覧の主催者など、投稿以外の著者。
  hostPubkeys?: readonly string[];
  /// 採用順位。空なら照会しない（この機能による非表示を行わない）。
  config: CommunityNodeConfig;
  /// 採用順位に選んだ node の認証・同意の状態。
  statuses?: readonly CommunityNodeNodeStatus[];
};

/// #1061: 表示中の著者を採用 CN へ一括照会し、折りたたみ判断を shell state へ置く。
///
/// - 採用順位が空なら照会しない。送るのは著者 pubkey だけで、投稿本文や閲覧履歴は送らない。
/// - 判断は評価の期限（ADR 0026 §8.4、既定 600 秒）まで使い、過ぎたら捨てて照会し直す。
/// - 採用順位・認証・必須同意が変わったら、すべての判断を捨てて照会し直す。
/// - 失敗は runtime 側が「未評価」として返すため、折りたたみは起きない（fail-open）。
export function useAuthorTrustGateLookup({
  api,
  posts,
  hostPubkeys = [],
  config,
  statuses = [],
}: UseAuthorTrustGateLookupArgs) {
  const storeApi = useDesktopShellStoreApi();
  const priority = config.trust_node_priority ?? [];
  const active = priority.length > 0;
  const adoptionSignature = trustGateAdoptionSignature(config, statuses);
  // 照会済みの著者と、その判断を使ってよい期限（epoch ms）。
  const expiryRef = useRef<Map<string, number>>(new Map());
  const queueRef = useRef<Set<string>>(new Set());
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const generationRef = useRef(0);
  const apiRef = useRef(api);
  const authorsRef = useRef<ReadonlySet<string>>(new Set<string>());
  useEffect(() => {
    apiRef.current = api;
  }, [api]);

  const authors = useMemo(() => {
    const unique = new Set<string>();
    for (const post of posts) {
      for (const author of postTrustGateAuthors(post)) unique.add(author);
    }
    for (const host of hostPubkeys) {
      const trimmed = host.trim();
      if (trimmed) unique.add(trimmed);
    }
    return unique;
  }, [hostPubkeys, posts]);

  // 採用順位・認証・必須同意が変わったら、以前の判断と照会済み記録を捨てる。
  useEffect(() => {
    generationRef.current += 1;
    expiryRef.current = new Map();
    queueRef.current = new Set();
    if (timerRef.current) {
      clearTimeout(timerRef.current);
      timerRef.current = null;
    }
    storeApi.getState().setField('authorTrustGates', {});
  }, [adoptionSignature, storeApi]);

  useEffect(() => {
    authorsRef.current = authors;
    if (!active) return;

    async function lookupBatch(batch: string[], generation: number) {
      const fallback = Date.now() + AUTHOR_TRUST_GATE_LOOKUP_FALLBACK_TTL_MS;
      try {
        const result = await apiRef.current.evaluateAuthorTrustGates({ author_pubkeys: batch });
        if (generation !== generationRef.current) return;
        const additions: Record<string, AuthorTrustGate> = {};
        for (const gate of result.gates) {
          additions[gate.author_pubkey] = gate;
          expiryRef.current.set(gate.author_pubkey, gateExpiry(gate, fallback));
        }
        if (Object.keys(additions).length === 0) return;
        storeApi.getState().setField('authorTrustGates', (current) => ({
          ...current,
          ...additions,
        }));
      } catch {
        // 照会できない著者は未評価のまま（折りたたまない）。一定時間後に照会し直す。
        if (generation !== generationRef.current) return;
        for (const author of batch) expiryRef.current.set(author, fallback);
      }
    }

    function flush() {
      timerRef.current = null;
      const batch = [...queueRef.current];
      queueRef.current = new Set();
      const generation = generationRef.current;
      for (let offset = 0; offset < batch.length; offset += AUTHOR_TRUST_GATE_LOOKUP_BATCH_SIZE) {
        void lookupBatch(
          batch.slice(offset, offset + AUTHOR_TRUST_GATE_LOOKUP_BATCH_SIZE),
          generation
        );
      }
    }

    /// 期限切れの判断を捨て、判断を持たない著者を照会待ちに入れる。
    function refresh() {
      const now = Date.now();
      const expired = [...expiryRef.current]
        .filter(([, expiresAt]) => expiresAt <= now)
        .map(([author]) => author);
      for (const author of expired) expiryRef.current.delete(author);
      if (expired.length > 0) {
        // 期限切れの判断はそのまま使わない（古い判断で折りたたみ続けない）。
        storeApi.getState().setField('authorTrustGates', (current) => {
          const next = { ...current };
          for (const author of expired) delete next[author];
          return next;
        });
      }
      for (const author of authorsRef.current) {
        if (expiryRef.current.has(author)) continue;
        // 照会中も期限として扱い、応答が返るまで同じ著者を二重に送らない。
        expiryRef.current.set(author, now + AUTHOR_TRUST_GATE_LOOKUP_FALLBACK_TTL_MS);
        queueRef.current.add(author);
      }
      // 直前の cleanup で timer が消えている場合も、待ち行列が残っていれば張り直す。
      if (queueRef.current.size === 0 || timerRef.current) return;
      timerRef.current = setTimeout(flush, AUTHOR_TRUST_GATE_LOOKUP_DEBOUNCE_MS);
    }

    refresh();
    const sweep = setInterval(refresh, AUTHOR_TRUST_GATE_LOOKUP_SWEEP_MS);

    return () => {
      clearInterval(sweep);
      if (timerRef.current) {
        clearTimeout(timerRef.current);
        timerRef.current = null;
      }
    };
  }, [active, authors, storeApi]);
}
