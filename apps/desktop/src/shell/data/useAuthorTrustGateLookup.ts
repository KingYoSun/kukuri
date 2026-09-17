import { useEffect, useMemo, useRef } from 'react';

import type { AuthorTrustGate, CommunityNodeConfig, DesktopApi, PostView } from '@/lib/api';
import { useDesktopShellStoreApi } from '@/shell/store';

/// 同じ描画更新でまとめて照会するための待ち時間。
export const AUTHOR_TRUST_GATE_LOOKUP_DEBOUNCE_MS = 300;
/// 1 回の照会で送る著者数の上限（CN 側の一括評価の上限と同じ）。
export const AUTHOR_TRUST_GATE_LOOKUP_BATCH_SIZE = 100;

/// #1061: 投稿カードから、表示判断の対象になる著者を集める。
///
/// 引用・repost では元投稿の著者も対象にする（ADR 0022 と同じ扱い）。
export function postTrustGateAuthors(post: PostView): string[] {
  const authors = [post.author_pubkey, post.repost_of?.source_author_pubkey];
  return authors
    .map((pubkey) => pubkey?.trim())
    .filter((pubkey): pubkey is string => Boolean(pubkey));
}

export type UseAuthorTrustGateLookupArgs = {
  api: Pick<DesktopApi, 'evaluateAuthorTrustGates'>;
  /// 表示中の投稿（タイムライン・スレッド・プロフィール・ブックマーク・見つける）。
  posts: readonly PostView[];
  /// live / game 一覧の主催者など、投稿以外の著者。
  hostPubkeys?: readonly string[];
  /// 採用順位。空なら照会しない（この機能による非表示を行わない）。
  config: CommunityNodeConfig;
};

/// #1061: 表示中の著者を採用 CN へ一括照会し、折りたたみ判断を shell state へ置く。
///
/// - 採用順位が空なら照会しない。送るのは著者 pubkey だけで、投稿本文や閲覧履歴は送らない。
/// - 著者ごとにセッション中 1 回。採用順位が変わったら結果を破棄して照会し直す。
/// - 失敗は runtime 側が「未評価」として返すため、折りたたみは起きない（fail-open）。
export function useAuthorTrustGateLookup({
  api,
  posts,
  hostPubkeys = [],
  config,
}: UseAuthorTrustGateLookupArgs) {
  const storeApi = useDesktopShellStoreApi();
  const priority = config.trust_node_priority ?? [];
  const prioritySignature = priority.join('|');
  const active = priority.length > 0;
  const requestedRef = useRef<Set<string>>(new Set());
  const queueRef = useRef<Set<string>>(new Set());
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const generationRef = useRef(0);
  const apiRef = useRef(api);
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

  // 採用順位が変わったら、以前の判断と照会済み記録を捨てる。
  useEffect(() => {
    generationRef.current += 1;
    requestedRef.current = new Set();
    queueRef.current = new Set();
    if (timerRef.current) {
      clearTimeout(timerRef.current);
      timerRef.current = null;
    }
    storeApi.getState().setField('authorTrustGates', {});
  }, [prioritySignature, storeApi]);

  useEffect(() => {
    if (!active) return;
    let queued = false;
    for (const author of authors) {
      if (requestedRef.current.has(author)) continue;
      requestedRef.current.add(author);
      queueRef.current.add(author);
      queued = true;
    }
    if (!queued || timerRef.current) return;
    timerRef.current = setTimeout(() => {
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
    }, AUTHOR_TRUST_GATE_LOOKUP_DEBOUNCE_MS);

    async function lookupBatch(batch: string[], generation: number) {
      try {
        const result = await apiRef.current.evaluateAuthorTrustGates({ author_pubkeys: batch });
        if (generation !== generationRef.current) return;
        const additions: Record<string, AuthorTrustGate> = {};
        for (const gate of result.gates) additions[gate.author_pubkey] = gate;
        if (Object.keys(additions).length === 0) return;
        storeApi.getState().setField('authorTrustGates', (current) => ({
          ...current,
          ...additions,
        }));
      } catch {
        // 照会できない著者は未評価のまま（折りたたまない）。
      }
    }

    return () => {
      if (timerRef.current) {
        clearTimeout(timerRef.current);
        timerRef.current = null;
      }
    };
  }, [active, authors, storeApi]);
}
