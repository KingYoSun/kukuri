import { normalizeInvokeError } from './invoke/error';

export type TrustRelationUnavailableReason =
  | 'trust_not_configured'
  | 'trust_not_activated'
  | 'relation_unavailable'
  | 'auth_required'
  | 'consent_required'
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
    default:
      return 'other';
  }
}

export function trustRelationErrorMessage(error: unknown): string {
  return normalizeInvokeError(error).message;
}
