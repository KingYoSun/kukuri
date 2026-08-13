import { normalizeInvokeError } from './invoke/error';

export type TrustRelationUnavailableReason =
  | 'trust_not_configured'
  | 'trust_not_activated'
  | 'relation_unavailable'
  | 'other';

export function trustRelationUnavailableReason(error: unknown): TrustRelationUnavailableReason {
  switch (normalizeInvokeError(error).code) {
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
