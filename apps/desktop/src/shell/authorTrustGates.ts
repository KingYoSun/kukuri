import type { AuthorTrustGate, PostView } from '@/lib/api';

/// #1061: 投稿カードに渡す表示判断（ADR 0026 §8.4）。
export type AuthorTrustGateView = {
  /// 折りたたむ対象の著者。引用・repost では元投稿の著者のこともある。
  authorPubkey: string;
  /// 判断に使った CN。
  nodeBaseUrl: string | null;
  /// 評価が下がった理由の種類。
  reasons: AuthorTrustGate['reasons'];
  /// 引用・repost の元投稿の著者による折りたたみか。
  fromRepostSource: boolean;
};

/// 投稿の著者と引用元の著者から、折りたたむべき判断を選ぶ。
///
/// どちらも非表示推奨なら、カードの著者を優先して示す（ADR 0022 の repost 非表示と同じ順序）。
export function resolvePostTrustGate(
  post: PostView,
  gates: Record<string, AuthorTrustGate>
): AuthorTrustGateView | null {
  const candidates: { pubkey: string | undefined; fromRepostSource: boolean }[] = [
    { pubkey: post.author_pubkey, fromRepostSource: false },
    { pubkey: post.repost_of?.source_author_pubkey, fromRepostSource: true },
  ];
  for (const candidate of candidates) {
    const pubkey = candidate.pubkey?.trim();
    if (!pubkey) continue;
    const gate = gates[pubkey];
    if (!gate?.hidden) continue;
    return {
      authorPubkey: pubkey,
      nodeBaseUrl: gate.node_base_url ?? null,
      reasons: gate.reasons,
      fromRepostSource: candidate.fromRepostSource,
    };
  }
  return null;
}
