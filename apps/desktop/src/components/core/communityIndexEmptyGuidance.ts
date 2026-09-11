// #960: 検索成功 0 件の空状態を、検索先・照合対象・考えられる理由・次の行動へ変換する純関数。
//
// CN は索引済み投稿の本文だけを全文照合し(cn-indexer `SEARCH_INDEX([text])`)、索引は
// operator の supported scope × safety `allow` に限られる(ADR 0025 §2.2 / §2.5)。
// #975: 索引状況 read API(自分の申請状態と対象が supported set か)の応答があれば、理由を断定文へ
// 切り替える。未取得・失敗時は #960 どおり「可能性」として示し、対象外とは断定しない。個々の投稿の
// 反映状況は読めないため、supported でも「まだ反映されていない可能性」は残す。
// 表示専用であり、外部送信・認証・同意・永続化を伴う action は持たない。
import type { IndexingRequestView, IndexingStatusResponse, TimelineScope } from '@/lib/api';

import type { CommunityIndexingTarget } from './CommunityIndexingRequestDialog';
import { summarizeCommunityIndexingStatus } from './communityIndexingStatus';

export type CommunityIndexEmptyOperation = 'search' | 'discovery' | 'recommendations';
export type CommunityIndexEmptyScope = 'topic' | 'channel' | 'explore';
export type CommunityIndexEmptyReason =
  | 'notIndexedYet'
  | 'outsideNodeScope'
  | 'pubkeyQuery'
  | 'notSupportedTarget'
  | 'requestPending'
  | 'requestRejected';

/** 索引状況 read API の取得状態(workspace が空状態ごとに取り直す)。 */
export type CommunityIndexEmptyIndexStatusInput =
  | { kind: 'loading' }
  | { kind: 'unknown' }
  | { kind: 'known'; response: IndexingStatusResponse };

/** 空状態に表示する索引状況。target は対象付き読取り(公開 topic)または自分の申請から確定した状態。 */
export type CommunityIndexEmptyIndexStatus =
  | { kind: 'loading' }
  | { kind: 'unknown' }
  /** 非公開チャンネルで自分の申請が無い場合。所属証明なしでは supported を読まないため、申請画面での確認を案内する。 */
  | { kind: 'privateUnverified' }
  | {
      kind: 'target';
      state: 'supported' | 'notSupported' | 'pending' | 'approved' | 'rejected';
    }
  | {
      kind: 'ownRequests';
      approved: string[];
      pending: string[];
      rejected: string[];
    };
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
  /** #975: 選択ノードでの索引状況。未取得なら null。 */
  indexStatus: CommunityIndexEmptyIndexStatus | null;
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
  indexStatus: indexStatusInput = null,
}: {
  mode: 'topic' | 'explore';
  operation: CommunityIndexEmptyOperation;
  query: string;
  activeTopic: string;
  activeTimelineScope: TimelineScope;
  activeChannelLabel?: string | null;
  canRequestIndexing: boolean;
  indexStatus?: CommunityIndexEmptyIndexStatusInput | null;
}): CommunityIndexEmptyGuidance {
  const scope: CommunityIndexEmptyScope =
    mode === 'explore' ? 'explore' : activeTimelineScope.kind === 'channel' ? 'channel' : 'topic';
  const authorPubkey = operation === 'search' ? communityIndexQueryAsPubkey(query) : null;
  const topicId = activeTopic.trim();
  // 状態確認の対象は申請導線の有無に関わらず決まる(申請 CTA だけが canRequestIndexing に依存する)。
  let statusTarget: CommunityIndexingTarget | null = null;
  if (scope !== 'explore' && topicId) {
    statusTarget =
      activeTimelineScope.kind === 'channel'
        ? {
            kind: 'private_channel',
            topicId,
            channelId: activeTimelineScope.channel_id,
            channelLabel: activeChannelLabel?.trim() || activeTimelineScope.channel_id,
          }
        : { kind: 'public_topic', topicId };
  }
  const indexStatus = resolveIndexStatus(indexStatusInput, statusTarget);
  const targetState = indexStatus?.kind === 'target' ? indexStatus.state : null;

  // 確定した状態があれば断定文へ切り替え、無ければ #960 の「可能性」を維持する。
  const stateReasons: CommunityIndexEmptyReason[] =
    targetState === 'supported' || targetState === 'approved'
      ? ['notIndexedYet']
      : targetState === 'notSupported'
        ? ['notSupportedTarget']
        : targetState === 'pending'
          ? ['requestPending']
          : targetState === 'rejected'
            ? ['requestRejected']
            : ['notIndexedYet', 'outsideNodeScope'];
  const reasons: CommunityIndexEmptyReason[] = authorPubkey
    ? ['pubkeyQuery', ...stateReasons]
    : stateReasons;

  // 申請済み・索引対象が確定した対象には申請 CTA を出さない(再申請は状態を変えない)。
  const indexingTarget =
    canRequestIndexing && statusTarget && (targetState === null || targetState === 'notSupported')
      ? statusTarget
      : null;

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
    indexStatus,
  };
}

function resolveIndexStatus(
  input: CommunityIndexEmptyIndexStatusInput | null,
  target: CommunityIndexingTarget | null
): CommunityIndexEmptyIndexStatus | null {
  if (!input) return null;
  if (input.kind !== 'known') return input;
  if (target) {
    const summary = summarizeCommunityIndexingStatus(input.response, target);
    if (summary.ownRequest) return { kind: 'target', state: summary.ownRequest.status };
    if (summary.supported === true) return { kind: 'target', state: 'supported' };
    if (summary.supported === false) return { kind: 'target', state: 'notSupported' };
    // 非公開チャンネルは所属証明なしでは supported を読まない。申請も無ければ申請画面での確認を案内する。
    return target.kind === 'private_channel' ? { kind: 'privateUnverified' } : { kind: 'unknown' };
  }
  const byStatus = (status: IndexingRequestView['status']) =>
    input.response.requests
      .filter((request) => request.status === status)
      .map((request) => request.target_id);
  return {
    kind: 'ownRequests',
    approved: byStatus('approved'),
    pending: byStatus('pending'),
    rejected: byStatus('rejected'),
  };
}
