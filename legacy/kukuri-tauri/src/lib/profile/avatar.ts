import type { FetchProfileAvatarResult, UploadProfileAvatarResult } from '@/lib/api/tauri';
import type { UserAvatarMetadata } from '@/stores/types';

const PROFILE_AVATAR_DOC_ID = 'profile_avatars';

export const MAX_PROFILE_AVATAR_BYTES = 2 * 1024 * 1024;

export function buildAvatarDataUrl(format: string, base64Data: string): string {
  return `data:${format};base64,${base64Data}`;
}

export function buildUserAvatarMetadata(
  npub: string,
  result: UploadProfileAvatarResult,
): UserAvatarMetadata {
  return buildUserAvatarMetadataFromPayload(npub, {
    blob_hash: result.blob_hash,
    format: result.format,
    size_bytes: result.size_bytes,
    access_level: result.access_level,
    share_ticket: result.share_ticket,
    doc_version: result.doc_version,
    updated_at: result.updated_at,
    content_sha256: result.content_sha256,
  });
}

export function buildUserAvatarMetadataFromFetch(
  npub: string,
  result: FetchProfileAvatarResult,
): UserAvatarMetadata {
  return buildUserAvatarMetadataFromPayload(npub, {
    blob_hash: result.blob_hash,
    format: result.format,
    size_bytes: result.size_bytes,
    access_level: result.access_level,
    share_ticket: result.share_ticket,
    doc_version: result.doc_version,
    updated_at: result.updated_at,
    content_sha256: result.content_sha256,
  });
}

interface AvatarMetadataPayload {
  blob_hash: string;
  format: string;
  size_bytes: number;
  access_level: UserAvatarMetadata['accessLevel'];
  share_ticket: string;
  doc_version: number;
  updated_at: string;
  content_sha256: string;
}

function buildUserAvatarMetadataFromPayload(
  npub: string,
  payload: AvatarMetadataPayload,
): UserAvatarMetadata {
  return {
    blobHash: payload.blob_hash,
    format: payload.format,
    sizeBytes: payload.size_bytes,
    accessLevel: payload.access_level,
    shareTicket: payload.share_ticket,
    docVersion: payload.doc_version,
    updatedAt: payload.updated_at,
    contentSha256: payload.content_sha256,
    nostrUri: createAvatarSchemeUri(npub, payload),
  };
}

function createAvatarSchemeUri(
  npub: string,
  result: Pick<AvatarMetadataPayload, 'blob_hash' | 'doc_version'>,
): string {
  const params = new URLSearchParams({
    npub,
    hash: result.blob_hash,
    v: result.doc_version.toString(),
  });
  return `iroh+avatar://${PROFILE_AVATAR_DOC_ID}?${params.toString()}`;
}
