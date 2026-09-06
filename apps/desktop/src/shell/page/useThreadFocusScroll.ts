import { useEffect, useRef } from 'react';
import { useShallow } from 'zustand/react/shallow';
import { useDesktopShellStore } from '@/shell/store';
import { activeWorkspaceColumn } from '@/shell/slices/workspace';

/** Only the selected thread owns this vertical scroll; matching timeline cards do not. */
export function useThreadFocusScroll(): void {
  const { column, objectId, requestId, posts } = useDesktopShellStore(useShallow((state) => {
    const active = activeWorkspaceColumn(state.workspaceState);
    return {
      column: active?.kind === 'thread' && active.entityId === state.selectedThread ? active : null,
      objectId: state.focusedObjectId,
      requestId: state.threadFocusRequestId,
      posts: state.threadsById[state.selectedThread ?? ''],
    };
  }));
  const completedRef = useRef<string | null>(null);
  const key = column && objectId ? JSON.stringify([column.id, objectId, requestId]) : null;
  useEffect(() => {
    if (!column || !objectId || !key || completedRef.current === key) return;
    const frame = window.requestAnimationFrame(() => {
      const surface = document.querySelector(`[data-column-id="${CSS.escape(column.id)}"]`);
      const body = surface?.querySelector<HTMLElement>('.shell-column-body');
      const post = body?.querySelector<HTMLElement>(`[data-post-object-id="${CSS.escape(objectId)}"]`);
      if (!post || !body || surface?.getAttribute('aria-current') !== 'true') return;
      const targetRect = post.getBoundingClientRect();
      const top = body.scrollTop + targetRect.top - body.getBoundingClientRect().top -
        Math.max(0, (body.clientHeight - targetRect.height) / 2);
      // Do not scroll horizontal ancestors or the document (especially on mobile).
      if (typeof body.scrollTo === 'function') body.scrollTo({ top: Math.max(0, top), behavior: 'instant' });
      else body.scrollTop = Math.max(0, top);
      post.focus({ preventScroll: true });
      completedRef.current = key;
    });
    return () => window.cancelAnimationFrame(frame);
  }, [column, objectId, key, posts]);
}
