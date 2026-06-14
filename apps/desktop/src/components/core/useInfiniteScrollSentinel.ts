import { useEffect, useRef } from 'react';

/**
 * Shared infinite-scroll sentinel used by the timeline and thread-tree feeds.
 *
 * Returns a ref to attach to a sentinel element rendered at the end of the list.
 * When the sentinel scrolls into view (and there is more to load), `onLoadMore`
 * fires. `canAutoLoad` reflects whether IntersectionObserver-driven auto-loading
 * is available; consumers fall back to a manual "Load more" button otherwise.
 */
export function useInfiniteScrollSentinel(options: {
  hasMore: boolean;
  loadingMore: boolean;
  onLoadMore?: () => void;
}): { sentinelRef: React.RefObject<HTMLDivElement | null>; canAutoLoad: boolean } {
  const { hasMore, loadingMore, onLoadMore } = options;
  const sentinelRef = useRef<HTMLDivElement | null>(null);
  const canAutoLoad =
    typeof window !== 'undefined' &&
    'IntersectionObserver' in window &&
    typeof onLoadMore === 'function';

  useEffect(() => {
    if (!canAutoLoad || !hasMore || loadingMore || !sentinelRef.current) {
      return;
    }
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) {
          onLoadMore?.();
        }
      },
      { rootMargin: '200px 0px' }
    );
    observer.observe(sentinelRef.current);
    return () => observer.disconnect();
  }, [canAutoLoad, hasMore, loadingMore, onLoadMore]);

  return { sentinelRef, canAutoLoad };
}
