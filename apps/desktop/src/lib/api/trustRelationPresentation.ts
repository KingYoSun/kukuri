import { normalizeInvokeError } from './invoke/error';

export type TrustRelationUnavailableReason =
  | 'trust_not_configured'
  | 'trust_not_activated'
  | 'relation_unavailable'
  | 'relation_visibility_not_configured'
  | 'relation_visibility_not_activated'
  | 'auth_required'
  | 'consent_required'
  | 'response_mismatch'
  | 'other';

export function trustRelationUnavailableReason(error: unknown): TrustRelationUnavailableReason {
  const normalized = normalizeInvokeError(error);
  // 索引画面と同じく、認証・同意の未達は安定コードで案内する(#705)。
  if (normalized.code === 'AUTH_REQUIRED' || normalized.status === 401) return 'auth_required';
  if (normalized.code === 'CONSENT_REQUIRED' || normalized.status === 403) return 'consent_required';
  switch (normalized.code) {
    case 'TRUST_READ_NOT_CONFIGURED':
      return 'trust_not_configured';
    case 'TRUST_READ_NOT_ACTIVATED':
      return 'trust_not_activated';
    case 'RELATION_NOT_FOUND':
      return 'relation_unavailable';
    // 距離利用停止(relation visibility)の未提供・失効(#712)。
    case 'RELATION_VISIBILITY_NOT_CONFIGURED':
      return 'relation_visibility_not_configured';
    case 'RELATION_VISIBILITY_NOT_ACTIVATED':
      return 'relation_visibility_not_activated';
    case 'TRUST_RELATION_RESPONSE_MISMATCH':
      // 応答本文の対象が要求した利用者と一致しない(#699)。内容は採用しない。
      return 'response_mismatch';
    default:
      return 'other';
  }
}

export function trustRelationErrorMessage(error: unknown): string {
  return normalizeInvokeError(error).message;
}
