import type { Post } from '@/stores/types';

export interface ThreadTreeNode {
  post: Post;
  children: ThreadTreeNode[];
}

export interface ThreadTreeBuildResult {
  root: ThreadTreeNode | null;
  detachedRoots: ThreadTreeNode[];
}

const sortNodeByCreatedAt = (left: ThreadTreeNode, right: ThreadTreeNode) =>
  left.post.created_at - right.post.created_at;

const cloneSortedTree = (node: ThreadTreeNode): ThreadTreeNode => ({
  post: node.post,
  children: node.children
    .map((child) => cloneSortedTree(child))
    .sort((left, right) => sortNodeByCreatedAt(left, right)),
});

export const buildThreadTree = (posts: Post[]): ThreadTreeBuildResult => {
  if (posts.length === 0) {
    return {
      root: null,
      detachedRoots: [],
    };
  }

  const orderedPosts = [...posts].sort((left, right) => left.created_at - right.created_at);
  const nodeMap = new Map<string, ThreadTreeNode>(
    orderedPosts.map((post) => [
      post.id,
      {
        post,
        children: [],
      },
    ]),
  );
  const rootCandidates: ThreadTreeNode[] = [];

  for (const post of orderedPosts) {
    const node = nodeMap.get(post.id);
    if (!node) {
      continue;
    }

    const parentId = post.threadParentEventId?.trim();
    const parentNode = parentId ? nodeMap.get(parentId) : undefined;

    if (parentNode && parentId !== post.id) {
      parentNode.children.push(node);
      continue;
    }

    rootCandidates.push(node);
  }

  const rootId =
    orderedPosts.find((post) => post.threadRootEventId === post.id)?.id ??
    orderedPosts.find((post) => !post.threadParentEventId)?.id ??
    rootCandidates[0]?.post.id;

  const root = rootId ? (nodeMap.get(rootId) ?? null) : null;

  if (!root) {
    return {
      root: null,
      detachedRoots: [],
    };
  }

  return {
    root: cloneSortedTree(root),
    detachedRoots: rootCandidates
      .filter((candidate) => candidate.post.id !== root.post.id)
      .map((candidate) => cloneSortedTree(candidate))
      .sort((left, right) => sortNodeByCreatedAt(left, right)),
  };
};
