// kukuri の topic ID は wire 上では `kukuri:topic:<name>` の名前空間付き文字列で扱う
// (gossip topic 導出・docs replica・community node の index scope と互換を保つ)。
// UI ではこの prefix を見せない: 入力は normalizeTopicId で ID へ、表示は
// topicDisplayName で名前へ変換し、この 2 関数以外で prefix を直接扱わない。

export const TOPIC_ID_PREFIX = 'kukuri:topic:';

/// topic ID から UI 表示名を得る。名前空間 prefix の無い（外部由来の）ID はそのまま返す。
export function topicDisplayName(topicId: string): string {
  return topicId.startsWith(TOPIC_ID_PREFIX) ? topicId.slice(TOPIC_ID_PREFIX.length) : topicId;
}

/// ユーザー入力を topic ID へ正規化する。prefix 付きの完全な ID（共有されたものの
/// 貼り付け等）はそのまま受け付ける。空入力は空文字を返し、呼び出し側で無視する。
export function normalizeTopicId(input: string): string {
  const trimmed = input.trim();
  if (!trimmed) {
    return '';
  }
  return trimmed.startsWith(TOPIC_ID_PREFIX) ? trimmed : `${TOPIC_ID_PREFIX}${trimmed}`;
}
