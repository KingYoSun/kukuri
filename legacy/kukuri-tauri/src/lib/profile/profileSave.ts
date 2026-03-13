import type { NostrMetadata } from '@/lib/api/nostr';
import type { UpdateUserProfileParams } from '@/lib/api/tauri';

interface BuildProfileSavePayloadInput {
  npub: string;
  name: string;
  displayName: string;
  about: string;
  picture: string;
  nip05: string;
  publicProfile: boolean;
  showOnlineStatus: boolean;
}

interface BuildProfileSavePayloadResult {
  localProfile: UpdateUserProfileParams;
  nostrMetadata: NostrMetadata;
  displayName: string;
}

const trimText = (value: string | null | undefined): string => value?.trim() ?? '';

const toOptionalText = (value: string): string | undefined => {
  const trimmed = trimText(value);
  return trimmed.length > 0 ? trimmed : undefined;
};

export function buildProfileSavePayload(
  input: BuildProfileSavePayloadInput,
): BuildProfileSavePayloadResult {
  const name = trimText(input.name);
  const displayName = trimText(input.displayName) || name;
  const about = trimText(input.about);
  const picture = trimText(input.picture);
  const nip05 = trimText(input.nip05);

  return {
    localProfile: {
      npub: input.npub,
      name,
      displayName,
      about,
      picture,
      nip05,
    },
    nostrMetadata: {
      name,
      display_name: displayName,
      about: toOptionalText(about),
      picture: toOptionalText(picture),
      nip05: toOptionalText(nip05),
      kukuri_privacy: {
        public_profile: input.publicProfile,
        show_online_status: input.showOnlineStatus,
      },
    },
    displayName,
  };
}

export function collectUniqueSaveErrors(errors: string[]): string[] {
  const seen = new Set<string>();
  const result: string[] = [];
  for (const value of errors) {
    const normalized = value.trim();
    if (!normalized || seen.has(normalized)) {
      continue;
    }
    seen.add(normalized);
    result.push(normalized);
  }
  return result;
}
