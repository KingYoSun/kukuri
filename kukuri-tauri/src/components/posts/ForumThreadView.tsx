import { useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { ChevronDown, ChevronRight } from 'lucide-react';
import type { Post } from '@/stores/types';
import { Button } from '@/components/ui/button';
import { PostCard } from '@/components/posts/PostCard';
import { buildThreadTree, type ThreadTreeNode } from '@/components/posts/forumThreadTree';
import { cn } from '@/lib/utils';

const countChildren = (node: ThreadTreeNode): number => {
  if (node.children.length === 0) {
    return 0;
  }

  return node.children.reduce((total, child) => total + 1 + countChildren(child), 0);
};

interface ThreadBranchProps {
  node: ThreadTreeNode;
  depth: number;
  collapsedNodeIds: Set<string>;
  onToggleCollapse: (postId: string) => void;
}

function ThreadBranch({ node, depth, collapsedNodeIds, onToggleCollapse }: ThreadBranchProps) {
  const { t } = useTranslation();
  const hasChildren = node.children.length > 0;
  const isCollapsed = collapsedNodeIds.has(node.post.id);

  return (
    <article className="space-y-3" data-testid={`forum-thread-node-${node.post.id}`}>
      <div className="flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
        <span data-testid={`forum-thread-depth-${node.post.id}`}>
          {t('topics.threadDepth', { depth })}
        </span>
        {hasChildren && (
          <Button
            type="button"
            variant="ghost"
            size="sm"
            className="h-7 gap-1 px-2 text-xs"
            onClick={() => onToggleCollapse(node.post.id)}
            data-testid={`forum-thread-toggle-${node.post.id}`}
          >
            {isCollapsed ? (
              <ChevronRight className="h-3.5 w-3.5" />
            ) : (
              <ChevronDown className="h-3.5 w-3.5" />
            )}
            <span>
              {isCollapsed
                ? t('topics.threadExpandReplies', { count: node.children.length })
                : t('topics.threadCollapseReplies', { count: countChildren(node) })}
            </span>
          </Button>
        )}
      </div>

      <PostCard post={node.post} data-testid={`forum-thread-post-${node.post.id}`} />

      {hasChildren && !isCollapsed && (
        <div
          className={cn('space-y-4 border-l border-border/70 pl-4', depth > 1 ? 'ml-2' : 'ml-1')}
          data-testid={`forum-thread-children-${node.post.id}`}
        >
          {node.children.map((child) => (
            <ThreadBranch
              key={child.post.id}
              node={child}
              depth={depth + 1}
              collapsedNodeIds={collapsedNodeIds}
              onToggleCollapse={onToggleCollapse}
            />
          ))}
        </div>
      )}
    </article>
  );
}

interface ForumThreadViewProps {
  threadUuid: string;
  posts: Post[];
}

export function ForumThreadView({ threadUuid, posts }: ForumThreadViewProps) {
  const { t } = useTranslation();
  const { root, detachedRoots } = useMemo(() => buildThreadTree(posts), [posts]);
  const [collapsedNodeIds, setCollapsedNodeIds] = useState<Set<string>>(() => new Set());

  useEffect(() => {
    setCollapsedNodeIds(new Set());
  }, [threadUuid]);

  const handleToggleCollapse = (postId: string) => {
    setCollapsedNodeIds((current) => {
      const next = new Set(current);
      if (next.has(postId)) {
        next.delete(postId);
      } else {
        next.add(postId);
      }
      return next;
    });
  };

  if (!root) {
    return null;
  }

  return (
    <section className="space-y-6" data-testid={`forum-thread-${threadUuid}`}>
      <section className="space-y-2" data-testid="forum-thread-root">
        <p className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
          {t('topics.threadRoot')}
        </p>
        <PostCard post={root.post} data-testid={`forum-thread-post-${root.post.id}`} />
      </section>

      <section className="space-y-4" data-testid="forum-thread-replies">
        <p className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
          {t('topics.timelineReplies', { count: posts.length - 1 })}
        </p>
        {root.children.length === 0 ? (
          <p className="text-sm text-muted-foreground" data-testid="forum-thread-no-replies">
            {t('topics.threadNoReplies')}
          </p>
        ) : (
          root.children.map((child) => (
            <ThreadBranch
              key={child.post.id}
              node={child}
              depth={1}
              collapsedNodeIds={collapsedNodeIds}
              onToggleCollapse={handleToggleCollapse}
            />
          ))
        )}
      </section>

      {detachedRoots.length > 0 && (
        <section
          className="space-y-4 rounded-lg border border-dashed p-4"
          data-testid="forum-thread-detached"
        >
          <p className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
            {t('topics.threadDetachedReplies')}
          </p>
          {detachedRoots.map((node) => (
            <ThreadBranch
              key={node.post.id}
              node={node}
              depth={1}
              collapsedNodeIds={collapsedNodeIds}
              onToggleCollapse={handleToggleCollapse}
            />
          ))}
        </section>
      )}
    </section>
  );
}
