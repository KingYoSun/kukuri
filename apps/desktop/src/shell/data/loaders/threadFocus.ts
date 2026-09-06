import type { DesktopApi, TimelineView } from '@/lib/api';
import { THREAD_TIMELINE_LIMIT } from '@/shell/pagination';
import { mergeUniquePosts } from '@/shell/data/timelineMerge';

/** Resolve a requested post using only pages of the already selected thread. */
export async function loadThreadForFocus(
  api: Pick<DesktopApi, 'listThread'>,
  topic: string,
  threadId: string,
  focusObjectId: string | null,
  isCurrent: () => boolean
): Promise<TimelineView | null> {
  if (!isCurrent()) return null;
  let page = await api.listThread(topic, threadId, null, THREAD_TIMELINE_LIMIT);
  const visited = new Set<string>();
  while (isCurrent() && focusObjectId &&
    !page.items.some((post) => post.object_id === focusObjectId) && page.next_cursor) {
    const cursor = page.next_cursor;
    const key = JSON.stringify([cursor.created_at, cursor.object_id]);
    // A repeated cursor cannot find another target; never loop on stale pages.
    if (visited.has(key)) {
      page = { ...page, next_cursor: null };
      break;
    }
    visited.add(key);
    const next = await api.listThread(topic, threadId, cursor, THREAD_TIMELINE_LIMIT);
    if (!isCurrent()) return null;
    page = { items: mergeUniquePosts(page.items, next.items), next_cursor: next.next_cursor };
  }
  return isCurrent() ? page : null;
}
