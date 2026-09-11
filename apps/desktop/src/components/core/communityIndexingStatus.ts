// #975: 索引状況 read API(`GET /v1/indexing/status`)の応答を、対象ごとの表示用 summary へ変換する純関数。
//
// `supported` は scope ゲート(supported set)だけを表し、個々の投稿が検索に出るかは safety ゲートと
// sync の反映に依存する(ADR 0025 §2.8)。応答は永続化せず、表示 state でだけ保持する。
import type { IndexingRequestView, IndexingStatusResponse } from '@/lib/api';
import { InvokeError } from '@/lib/api/invoke/error';

import type { CommunityIndexingTarget } from './CommunityIndexingRequestDialog';

export type CommunityIndexingStatusSummary = {
  /** supported set に含まれるか。target 無指定(非公開チャンネルの所属証明送信前など)は null = 未確認。 */
  supported: boolean | null;
  /** 呼出し主自身の申請。無ければ null。 */
  ownRequest: IndexingRequestView | null;
};

export function communityIndexingTargetId(target: CommunityIndexingTarget): string {
  return target.kind === 'private_channel' ? target.channelId : target.topicId;
}

export function summarizeCommunityIndexingStatus(
  response: IndexingStatusResponse,
  target: CommunityIndexingTarget
): CommunityIndexingStatusSummary {
  const targetId = communityIndexingTargetId(target);
  const ownRequest =
    response.requests.find(
      (request) => request.scope_kind === target.kind && request.target_id === targetId
    ) ?? null;
  const supported =
    response.target &&
    response.target.scope_kind === target.kind &&
    response.target.scope_id === targetId
      ? response.target.supported
      : null;
  return { supported, ownRequest };
}

/**
 * 索引申請・索引状況の失敗を `shell:indexingRequest.errors.*` のキーへ写す。
 * 安定コードを status より先に見る(403 は同意不足と所属証明不足の両方があり得る)。
 */
export function communityIndexingErrorKey(
  error: unknown,
  fallback: 'requestFailed' | 'statusFailed'
): string {
  if (!(error instanceof InvokeError)) return fallback;
  // #713: 索引を提供しない・停止中のノードは申請も状態読取りも受け付けない(サーバ側の門)。
  if (error.code === 'INDEXING_REQUEST_NOT_CONFIGURED') return 'requestNotConfigured';
  if (error.code === 'INDEXING_REQUEST_NOT_ACTIVATED') return 'requestNotActivated';
  if (error.code === 'CHANNEL_INDEXING_NOT_CONFIGURED') return 'notConfigured';
  if (error.code === 'CHANNEL_SECRET_CONFLICT') return 'secretConflict';
  // #711: 所属証明の未提示・不一致・未登録は同一コード。
  if (error.code === 'CHANNEL_MEMBERSHIP_REQUIRED') return 'membershipRequired';
  if (error.code === 'PRIVATE_CHANNEL_CAPABILITY_UNAVAILABLE') return 'capabilityUnavailable';
  if (error.code === 'AUTH_REQUIRED' || error.status === 401) return 'authRequired';
  if (error.code === 'CONSENT_REQUIRED' || error.status === 403) return 'consentRequired';
  return fallback;
}
