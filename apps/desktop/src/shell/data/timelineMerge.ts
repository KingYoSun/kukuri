import type { PostView } from '@/lib/api';

export function postIdentityKey(post: Pick<PostView, 'object_id' | 'server_object_id'>): string {
  return post.server_object_id ?? post.object_id;
}

export function uniquePostsByIdentity(posts: PostView[]): PostView[] {
  const seen = new Set<string>();
  const nextPosts: PostView[] = [];

  for (const post of posts) {
    const key = postIdentityKey(post);
    if (seen.has(key)) {
      continue;
    }
    seen.add(key);
    nextPosts.push(post);
  }

  return nextPosts;
}

export function mergeUniquePosts(current: PostView[], incoming: PostView[]): PostView[] {
  const seen = new Set(current.map((post) => post.object_id));
  return [...current, ...incoming.filter((post) => !seen.has(post.object_id))];
}

export function hasLoadedOlderAuthoritativePosts(
  current: PostView[],
  incoming: PostView[]
): boolean {
  return current.filter((post) => !post.local_state).length > incoming.length;
}

export function mergeRefreshedVisiblePosts(
  current: PostView[],
  incoming: PostView[],
  preserveOlderPages: boolean
): PostView[] {
  const authoritativeIds = new Set(incoming.map((post) => postIdentityKey(post)));
  const localPosts = current.filter((post) => {
    if (!post.local_state) {
      return false;
    }
    const authoritativeId = postIdentityKey(post);
    return !authoritativeIds.has(authoritativeId);
  });
  const nextPosts = [...localPosts];
  const seenPostIds = new Set(nextPosts.map((post) => postIdentityKey(post)));

  for (const post of incoming) {
    const postId = postIdentityKey(post);
    if (seenPostIds.has(postId)) {
      continue;
    }
    nextPosts.push(post);
    seenPostIds.add(postId);
  }

  if (!preserveOlderPages) {
    return nextPosts;
  }

  for (const post of current) {
    if (post.local_state) {
      continue;
    }
    const authoritativeId = postIdentityKey(post);
    if (authoritativeIds.has(authoritativeId) || seenPostIds.has(authoritativeId)) {
      continue;
    }
    nextPosts.push(post);
    seenPostIds.add(authoritativeId);
  }

  return nextPosts;
}
