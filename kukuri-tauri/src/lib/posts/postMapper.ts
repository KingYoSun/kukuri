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
  };

  const author = applyKnownUserMetadata(baseAuthor);

  return {
    id: apiPost.id,
    content: apiPost.content,
    author,
    topicId: apiPost.topic_id,
    created_at: apiPost.created_at,
    tags: [],
    likes: apiPost.likes,
    boosts: apiPost.boosts ?? 0,
    replies: [],
    isSynced: apiPost.is_synced ?? true,
  };
}

export function enrichPostAuthorMetadata(post: Post): Post {
  const enrichedAuthor = applyKnownUserMetadata(post.author);

  if (enrichedAuthor === post.author) {
    return post;
  }

  return {
    ...post,
    author: enrichedAuthor,
    replies: post.replies?.map(enrichPostAuthorMetadata) ?? [],
  };
}
