// #960: 検索成功 0 件の空状態を、検索先・照合対象・考えられる理由・次の行動へ変換する純関数。
//
// CN は索引済み投稿の本文だけを全文照合し(cn-indexer `SEARCH_INDEX([text])`)、索引は
// operator の supported scope × safety `allow` に限られる(ADR 0025 §2.2 / §2.5)。索引の反映状況を
// 読む API は無いため、ここでは断定せず「可能性」と「確認できないこと」を示す。
// 表示専用であり、外部送信・認証・同意・永続化を伴う action は持たない。
import type { TimelineScope } from '@/lib/api';

import type { CommunityIndexingTarget } from './CommunityIndexingRequestDialog';

export type CommunityIndexEmptyOperation = 'search' | 'discovery' | 'recommendations';
export type CommunityIndexEmptyScope = 'topic' | 'channel' | 'explore';
export type CommunityIndexEmptyReason = 'notIndexedYet' | 'outsideNodeScope' | 'pubkeyQuery';
export type CommunityIndexEmptyAction =
  | 'openAuthor'
  | 'retry'
  | 'openTimeline'
  | 'requestIndexing'
  | 'openCommunityNodeSettings'
  | 'openConnectivity';

export type CommunityIndexEmptyGuidance = {
  scope: CommunityIndexEmptyScope;
  operation: CommunityIndexEmptyOperation;
  /** search で、検索語が投稿本文へ照合されたことを説明する。pubkey 入力時は代わりに pubkeyQuery 理由を出す。 */
  explainsPostTextMatching: boolean;
  reasons: CommunityIndexEmptyReason[];
  actions: CommunityIndexEmptyAction[];
  /** 検索語が公開鍵(64 桁 hex)のときだけ、正規化した値を持つ。 */
  authorPubkey: string | null;
  /** 索引登録申請の対象。topic mode で申請導線がある場合だけ持つ。 */
  indexingTarget: CommunityIndexingTarget | null;
  /** 横断検索では対象 topic が一意でないため、トピック一覧からの申請を文言で案内する。 */
  explainsTopicListRequest: boolean;
};

const PUBKEY_PATTERN = /^[0-9a-f]{64}$/i;

export function communityIndexQueryAsPubkey(query: string): string | null {
  const trimmed = query.trim();
  return PUBKEY_PATTERN.test(trimmed) ? trimmed.toLowerCase() : null;
}

export function communityIndexEmptyGuidance({
  mode,
  operation,
  query,
  activeTopic,
  activeTimelineScope,
  activeChannelLabel,
  canRequestIndexing,
}: {
  mode: 'topic' | 'explore';
  operation: CommunityIndexEmptyOperation;
  query: string;
  activeTopic: string;
  activeTimelineScope: TimelineScope;
  activeChannelLabel?: string | null;
  canRequestIndexing: boolean;
}): CommunityIndexEmptyGuidance {
  const scope: CommunityIndexEmptyScope =
    mode === 'explore' ? 'explore' : activeTimelineScope.kind === 'channel' ? 'channel' : 'topic';
  const authorPubkey = operation === 'search' ? communityIndexQueryAsPubkey(query) : null;
  const reasons: CommunityIndexEmptyReason[] = authorPubkey
    ? ['pubkeyQuery', 'notIndexedYet', 'outsideNodeScope']
    : ['notIndexedYet', 'outsideNodeScope'];

  const topicId = activeTopic.trim();
  let indexingTarget: CommunityIndexingTarget | null = null;
  if (canRequestIndexing && scope !== 'explore' && topicId) {
    if (activeTimelineScope.kind === 'channel') {
      indexingTarget = {
        kind: 'private_channel',
        topicId,
        channelId: activeTimelineScope.channel_id,
        channelLabel: activeChannelLabel?.trim() || activeTimelineScope.channel_id,
      };
    } else {
      indexingTarget = { kind: 'public_topic', topicId };
    }
  }

  const actions: CommunityIndexEmptyAction[] = [];
  if (authorPubkey) actions.push('openAuthor');
  actions.push('retry', 'openTimeline');
  if (indexingTarget) actions.push('requestIndexing');
  actions.push('openCommunityNodeSettings', 'openConnectivity');

  return {
    scope,
    operation,
    explainsPostTextMatching: operation === 'search' && !authorPubkey,
    reasons,
    actions,
    authorPubkey,
    indexingTarget,
    explainsTopicListRequest: scope === 'explore' && canRequestIndexing,
  };
}
