import { type PostCardView } from './types';

export type ThreadTreeNode = {
  view: PostCardView;
  depth: number;
  /**
   * Vertical guide rails for the ancestor columns above this node's own connector
   * column (one entry per depth 1..depth-1). `true` means that ancestor has a
   * following sibling, so a continuing `│` line is drawn in that gutter column.
   */
  rails: boolean[];
  /** Whether this node is the last among its siblings (renders `└` instead of `├`). */
  isLast: boolean;
};

/**
 * Build a reply tree from the flat thread post list and flatten it in DFS
 * pre-order so the existing keyed `<li>` / focus-highlight rendering still works.
 *
 * - Parent is resolved via `reply_to` against any of a post's ids
 *   (object_id / server_object_id / local_id).
 * - Posts whose parent is not loaded (orphans) and the thread root attach at depth 0.
 * - Siblings and roots are ordered by `created_at` ascending (stable).
 * - Cycles are broken: every post is emitted exactly once.
 */
export function buildThreadTree(posts: PostCardView[]): ThreadTreeNode[] {
  const byId = new Map<string, PostCardView>();
  for (const view of posts) {
    const post = view.post;
    byId.set(post.object_id, view);
    if (post.server_object_id) {
      byId.set(post.server_object_id, view);
    }
    if (post.local_id) {
      byId.set(post.local_id, view);
    }
  }

  const parentOf = new Map<string, string | null>();
  const childrenOf = new Map<string, PostCardView[]>();
  for (const view of posts) {
    const post = view.post;
    const replyTo = post.reply_to ?? null;
    let parent: PostCardView | null = null;
    if (replyTo) {
      const candidate = byId.get(replyTo) ?? null;
      if (candidate && candidate.post.object_id !== post.object_id) {
        parent = candidate;
      }
    }
    parentOf.set(post.object_id, parent ? parent.post.object_id : null);
    if (parent) {
      const list = childrenOf.get(parent.post.object_id) ?? [];
      list.push(view);
      childrenOf.set(parent.post.object_id, list);
    }
  }

  const byCreatedAt = (a: PostCardView, b: PostCardView) =>
    a.post.created_at - b.post.created_at;
  for (const list of childrenOf.values()) {
    list.sort(byCreatedAt);
  }

  const roots = posts
    .filter((view) => parentOf.get(view.post.object_id) == null)
    .sort(byCreatedAt);

  const result: ThreadTreeNode[] = [];
  const visited = new Set<string>();
  const visit = (view: PostCardView, depth: number, rails: boolean[], isLast: boolean) => {
    const id = view.post.object_id;
    if (visited.has(id)) {
      return;
    }
    visited.add(id);
    result.push({ view, depth, rails, isLast });
    const children = childrenOf.get(id) ?? [];
    // Roots (depth 0) have no gutter column, so their children start with no rails;
    // deeper nodes carry this node's column as a pass-through rail when it has a
    // following sibling.
    const childRails = depth === 0 ? [] : [...rails, !isLast];
    children.forEach((child, index) => {
      visit(child, depth + 1, childRails, index === children.length - 1);
    });
  };

  roots.forEach((root, index) => visit(root, 0, [], index === roots.length - 1));
  // Any post left unvisited (e.g. trapped in a reply cycle) is promoted to a root.
  const orphans = posts
    .filter((view) => !visited.has(view.post.object_id))
    .sort(byCreatedAt);
  orphans.forEach((orphan, index) => visit(orphan, 0, [], index === orphans.length - 1));

  return result;
}
