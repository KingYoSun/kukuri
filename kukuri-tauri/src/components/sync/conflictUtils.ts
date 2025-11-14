import type { OfflineAction } from '@/types/offline';

export type DocConflictDetails = {
  docVersion?: number;
  blobHash?: string;
  payloadBytes?: number;
  format?: string;
  shareTicket?: string;
};

export function toNumber(value: unknown): number | undefined {
  if (typeof value === 'number') {
    return Number.isNaN(value) ? undefined : value;
  }

  if (typeof value === 'string') {
    const parsed = Number(value);
    return Number.isNaN(parsed) ? undefined : parsed;
  }

  return undefined;
}

function parseActionPayload(action?: OfflineAction): Record<string, unknown> | null {
  if (!action) {
    return null;
  }

  const raw = action.actionData;

  if (typeof raw === 'string') {
    try {
      return JSON.parse(raw) as Record<string, unknown>;
    } catch {
      return null;
    }
  }

  if (raw && typeof raw === 'object') {
    return raw as Record<string, unknown>;
  }

  return null;
}

export function extractDocConflictDetails(action?: OfflineAction): DocConflictDetails | null {
  const payload = parseActionPayload(action);

  if (!payload) {
    return null;
  }

  const docVersion = toNumber(payload['docVersion'] ?? payload['doc_version']);
  const payloadBytes = toNumber(
    payload['payloadBytes'] ??
      payload['payload_bytes'] ??
      payload['sizeBytes'] ??
      payload['size_bytes'],
  );
  const blobHashCandidate = payload['blobHash'] ?? payload['blob_hash'];
  const formatCandidate = payload['format'] ?? payload['mimeType'];
  const shareTicketCandidate = payload['shareTicket'] ?? payload['share_ticket'];

  const blobHash = typeof blobHashCandidate === 'string' ? blobHashCandidate : undefined;
  const format = typeof formatCandidate === 'string' ? formatCandidate : undefined;
  const shareTicket = typeof shareTicketCandidate === 'string' ? shareTicketCandidate : undefined;

  if (
    typeof docVersion === 'undefined' &&
    !blobHash &&
    typeof payloadBytes === 'undefined' &&
    !format &&
    !shareTicket
  ) {
    return null;
  }

  return {
    docVersion,
    blobHash,
    payloadBytes,
    format,
    shareTicket,
  };
}

export function truncateMiddle(value: string, maxLength = 32) {
  if (value.length <= maxLength) {
    return value;
  }

  const keep = Math.max(4, Math.floor((maxLength - 3) / 2));
  return `${value.slice(0, keep)}...${value.slice(-keep)}`;
}

export function formatBytesValue(bytes?: number) {
  if (bytes === undefined || Number.isNaN(bytes)) {
    return undefined;
  }

  if (bytes < 1024) {
    return `${bytes} B`;
  }

  const units = ['KB', 'MB', 'GB', 'TB'];
  let value = bytes;
  let unitIndex = -1;

  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024;
    unitIndex += 1;
  }

  return `${value.toFixed(1)} ${units[Math.max(unitIndex, 0)]}`;
}
