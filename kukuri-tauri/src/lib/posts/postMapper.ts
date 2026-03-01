import { TauriApi, type Post as ApiPost } from '@/lib/api/tauri';
import type { Post, User } from '@/stores/types';
import { pubkeyToNpub } from '@/lib/utils/nostr';
import { applyKnownUserMetadata } from '@/lib/profile/userMetadata';
import { mapUserProfileToUser } from '@/lib/profile/profileMapper';

const AUTHOR_PROFILE_MISS_TTL_MS = 60_000;
const authorProfileCache = new Map<string, User>();
const authorProfileInFlight = new Map<string, Promise<User | null>>();
const authorProfileMissedAt = new Map<string, number>();

const shortenAuthorLabel = (value: string): string => {
  const trimmed = value.trim();
  if (!trimmed) {
    return 'P2P user';
  }
  if (trimmed.length <= 16) {
    return trimmed;
  }
  return `${trimmed.slice(0, 8)}...${trimmed.slice(-4)}`;
};

const normalizeCacheKey = (value: string): string => value.trim().toLowerCase();

const resolveAuthorProfile = async (
  authorPubkey: string,
  authorNpub: string,
): Promise<User | null> => {
  const cacheKey = normalizeCacheKey(authorPubkey);
  const cached = authorProfileCache.get(cacheKey);
  if (cached) {
    return cached;
  }

  const missAt = authorProfileMissedAt.get(cacheKey);
  if (missAt && Date.now() - missAt < AUTHOR_PROFILE_MISS_TTL_MS) {
    return null;
  }

  const inFlight = authorProfileInFlight.get(cacheKey);
  if (inFlight) {
    return await inFlight;
  }

  const loader = (async (): Promise<User | null> => {
    try {
      const byPubkey = await TauriApi.getUserProfileByPubkey(authorPubkey);
      const byNpub =
        !byPubkey && authorNpub.startsWith('npub1')
          ? await TauriApi.getUserProfile(authorNpub)
          : null;
      const profile = byPubkey ?? byNpub;
      if (!profile) {
        authorProfileMissedAt.set(cacheKey, Date.now());
        return null;
      }
      const mapped = mapUserProfileToUser(profile);
      authorProfileCache.set(cacheKey, mapped);
      authorProfileMissedAt.delete(cacheKey);
      return mapped;
    } catch {
      authorProfileMissedAt.set(cacheKey, Date.now());
      return null;
    }
  })();

  authorProfileInFlight.set(cacheKey, loader);
  try {
    return await loader;
  } finally {
    authorProfileInFlight.delete(cacheKey);
  }
};

export async function mapPostResponseToDomain(apiPost: ApiPost): Promise<Post> {
  const npub =
    apiPost.author_npub && apiPost.author_npub.length > 0
      ? apiPost.author_npub
      : await pubkeyToNpub(apiPost.author_pubkey);
  const fallbackAuthorLabel = shortenAuthorLabel(npub || apiPost.author_pubkey);
  const resolvedProfile = await resolveAuthorProfile(apiPost.author_pubkey, npub);

  const baseAuthor: User = {
    id: apiPost.author_pubkey,
    pubkey: apiPost.author_pubkey,
    npub,
    name: fallbackAuthorLabel,
    displayName: fallbackAuthorLabel,
    about: '',
    picture: '',
    nip05: '',
    avatar: null,
    publicProfile: true,
    showOnlineStatus: false,
  };

  const author = applyKnownUserMetadata(
    resolvedProfile
      ? {
          ...resolvedProfile,
          id: apiPost.author_pubkey,
          pubkey: apiPost.author_pubkey,
          npub: resolvedProfile.npub || npub,
          name: resolvedProfile.name?.trim() ? resolvedProfile.name : fallbackAuthorLabel,
          displayName: resolvedProfile.displayName?.trim()
            ? resolvedProfile.displayName
            : resolvedProfile.name?.trim()
              ? resolvedProfile.name
              : fallbackAuthorLabel,
        }
      : baseAuthor,
  );

  return {
    id: apiPost.id,
    content: apiPost.content,
    author,
    topicId: apiPost.topic_id,
    threadNamespace: apiPost.thread_namespace ?? null,
    threadUuid: apiPost.thread_uuid ?? null,
    threadRootEventId: apiPost.thread_root_event_id ?? null,
    threadParentEventId: apiPost.thread_parent_event_id ?? null,
    scope: (apiPost.scope ?? 'public') as Post['scope'],
    epoch: apiPost.epoch ?? null,
    isEncrypted: apiPost.is_encrypted ?? false,
    created_at: apiPost.created_at,
    tags: [],
    likes: apiPost.likes,
    boosts: apiPost.boosts ?? 0,
    replies: [],
    replyCount: apiPost.replies ?? 0,
    isSynced: apiPost.is_synced ?? true,
  };
}

export function enrichPostAuthorMetadata(post: Post): Post {
  const enrichedAuthor = applyKnownUserMetadata(post.author);

  if (enrichedAuthor === post.author) {
    return post;
  }

  const repliesSource = Array.isArray(post.replies) ? post.replies : [];

  return {
    ...post,
    author: enrichedAuthor,
    replies: repliesSource.map(enrichPostAuthorMetadata),
  };
}
