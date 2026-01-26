import type { Post as ApiPost } from '@/lib/api/tauri';
import type { Post, User } from '@/stores/types';
import { pubkeyToNpub } from '@/lib/utils/nostr';
import { applyKnownUserMetadata } from '@/lib/profile/userMetadata';

export async function mapPostResponseToDomain(apiPost: ApiPost): Promise<Post> {
  const npub =
    apiPost.author_npub && apiPost.author_npub.length > 0
      ? apiPost.author_npub
      : await pubkeyToNpub(apiPost.author_pubkey);

  const baseAuthor: User = {
    id: apiPost.author_pubkey,
    pubkey: apiPost.author_pubkey,
    npub,
    name: '',
    displayName: '',
    about: '',
    picture: '',
    nip05: '',
    avatar: null,
    publicProfile: true,
    showOnlineStatus: false,
  };

  const author = applyKnownUserMetadata(baseAuthor);

  return {
    id: apiPost.id,
    content: apiPost.content,
    author,
    topicId: apiPost.topic_id,
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
