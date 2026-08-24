import {
  Children,
  useCallback,
  useEffect,
  useRef,
  useState,
  type DragEvent,
  type ReactNode,
} from 'react';
import { prefersReducedMotion } from '@/lib/reducedMotion';
import { columnCanvasEdgeScrollDirection } from './columnCanvasGeometry';
import { nearestColumnToViewportCenter } from './columnPagingGeometry';

type ColumnCanvasProps = {
  activeColumnId: string;
  children: ReactNode;
  columnIds?: string[];
  label?: string;
  onActivateColumn: (columnId: string, syncRoute: boolean) => void;
  onMoveColumn?: (columnId: string, targetIndex: number) => void;
  onVisibleColumnIdsChange?: (columnIds: string[]) => void;
};

const EDGE_SCROLL_STEP_PX = 18;
const MOBILE_QUERY = '(max-width: 759px)';
const SCROLL_SETTLE_MS = 120;

function isMobileViewport() {
  return window.matchMedia?.(MOBILE_QUERY).matches ?? false;
}

function findColumnId(target: EventTarget | null) {
  if (!(target instanceof Element)) return null;
  return target.closest<HTMLElement>('[data-column-id]')?.dataset.columnId ?? null;
}

function isInteractiveTarget(target: EventTarget | null) {
  if (!(target instanceof Element)) return false;
  return Boolean(
    target.closest('button, a, input, textarea, select, [role="button"], [role="link"]')
  );
}

export function ColumnCanvas({
  activeColumnId,
  children,
  columnIds = [],
  label = 'Column workspace',
  onActivateColumn,
  onMoveColumn,
  onVisibleColumnIdsChange,
}: ColumnCanvasProps) {
  const canvasRef = useRef<HTMLDivElement | null>(null);
  const previousActiveColumnIdRef = useRef<string | null>(null);
  const autoScrollFrameRef = useRef<number | null>(null);
  const autoScrollDirectionRef = useRef<-1 | 0 | 1>(0);
  const draggingColumnIdRef = useRef<string | null>(null);
  const scrollSettleTimeoutRef = useRef<number | null>(null);
  const [dropTarget, setDropTarget] = useState<{ index: number; left: number } | null>(null);
  const [announcement, setAnnouncement] = useState('');

  const stopAutoScroll = useCallback(() => {
    autoScrollDirectionRef.current = 0;
    if (autoScrollFrameRef.current !== null) {
      window.cancelAnimationFrame(autoScrollFrameRef.current);
      autoScrollFrameRef.current = null;
    }
  }, []);

  const startAutoScroll = useCallback((direction: -1 | 1) => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    autoScrollDirectionRef.current = direction;
    canvas.scrollLeft += direction * EDGE_SCROLL_STEP_PX;
    if (prefersReducedMotion()) {
      return;
    }
    if (autoScrollFrameRef.current !== null) return;
    const tick = () => {
      const current = canvasRef.current;
      const scrollDirection = autoScrollDirectionRef.current;
      if (!current || scrollDirection === 0) {
        autoScrollFrameRef.current = null;
        return;
      }
      current.scrollLeft += scrollDirection * EDGE_SCROLL_STEP_PX;
      autoScrollFrameRef.current = window.requestAnimationFrame(tick);
    };
    autoScrollFrameRef.current = window.requestAnimationFrame(tick);
  }, []);

  const resetDrag = useCallback(() => {
    stopAutoScroll();
    draggingColumnIdRef.current = null;
    setDropTarget(null);
  }, [stopAutoScroll]);

  useEffect(() => resetDrag, [resetDrag]);

  useEffect(() => () => {
    if (scrollSettleTimeoutRef.current !== null) {
      window.clearTimeout(scrollSettleTimeoutRef.current);
    }
  }, []);

  useEffect(() => {
    if (previousActiveColumnIdRef.current === activeColumnId) return;
    previousActiveColumnIdRef.current = activeColumnId;
    const column = canvasRef.current?.querySelector<HTMLElement>(
      `[data-column-id="${CSS.escape(activeColumnId)}"]`
    );
    if (typeof column?.scrollIntoView === 'function') {
      const mobile = isMobileViewport();
      const reducedMotion = prefersReducedMotion();
      column.scrollIntoView({
        block: 'nearest',
        inline: mobile ? 'center' : 'nearest',
        ...(mobile ? { behavior: reducedMotion ? 'auto' : 'smooth' } : {}),
      });
    }
    if (column && (!document.activeElement || document.activeElement === document.body)) {
      const focusFrameId = window.requestAnimationFrame(() => {
        if (!document.activeElement || document.activeElement === document.body) {
          column.focus({ preventScroll: true });
        }
      });
      return () => window.cancelAnimationFrame(focusFrameId);
    }
  }, [activeColumnId]);

  const columnCount = Children.count(children);
  // Issue #765: 同数の transient 置換(Column 数は不変で id 列だけ入れ替わる)でも
  // 新しい Column 要素を observe し直すため、id 列を安定 key にして再購読する。
  // visible 集合は effect ローカルなので、再購読時に作り直され置換前の id は残留しない。
  const columnIdsKey = columnIds.join('\u0000');
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || !onVisibleColumnIdsChange) return;
    const columns = Array.from(canvas.querySelectorAll<HTMLElement>('[data-column-id]'));
    if (typeof IntersectionObserver === 'undefined') {
      onVisibleColumnIdsChange(columns.map((column) => column.dataset.columnId!).filter(Boolean));
      return;
    }
    const visible = new Set<string>();
    const publish = () => onVisibleColumnIdsChange(
      columns.flatMap((column) => {
        const id = column.dataset.columnId;
        return id && visible.has(id) ? [id] : [];
      })
    );
    const observer = new IntersectionObserver((entries) => {
      for (const entry of entries) {
        const id = (entry.target as HTMLElement).dataset.columnId;
        if (!id) continue;
        if (entry.isIntersecting && entry.intersectionRatio > 0.01) visible.add(id);
        else visible.delete(id);
      }
      publish();
    }, { root: canvas, threshold: [0, 0.01, 0.5, 1] });
    columns.forEach((column) => observer.observe(column));
    return () => observer.disconnect();
  }, [columnCount, columnIdsKey, onVisibleColumnIdsChange]);

  const settleMobileScroll = () => {
    if (!isMobileViewport()) return;
    if (scrollSettleTimeoutRef.current !== null) {
      window.clearTimeout(scrollSettleTimeoutRef.current);
    }
    scrollSettleTimeoutRef.current = window.setTimeout(() => {
      scrollSettleTimeoutRef.current = null;
      const canvas = canvasRef.current;
      if (!canvas) return;
      const columns = Array.from(
        canvas.querySelectorAll<HTMLElement>('[data-column-id]')
      ).flatMap((column) => {
        const id = column.dataset.columnId;
        return id ? [{ id, left: column.offsetLeft, width: column.offsetWidth }] : [];
      });
      const nearest = nearestColumnToViewportCenter(columns, canvas.scrollLeft, canvas.clientWidth);
      if (nearest && nearest !== activeColumnId) onActivateColumn(nearest, true);
    }, SCROLL_SETTLE_MS);
  };

  const activateFromEvent = (target: EventTarget | null) => {
    if (target instanceof Element && target.closest('[data-column-gesture-owner]')) return;
    const columnId = findColumnId(target);
    if (columnId && columnId !== activeColumnId) {
      onActivateColumn(columnId, !isInteractiveTarget(target));
    }
  };

  const calculateDropTarget = (clientX: number, draggedColumnId: string) => {
    const canvas = canvasRef.current;
    if (!canvas) return null;
    const columns = Array.from(
      canvas.querySelectorAll<HTMLElement>('[data-column-id]')
    ).filter((column) => column.dataset.columnId !== draggedColumnId);
    let index = columns.findIndex((column) => {
      const rect = column.getBoundingClientRect();
      return clientX < rect.left + rect.width / 2;
    });
    if (index < 0) index = columns.length;
    const anchor = columns[index];
    const previous = columns[index - 1];
    const left = anchor
      ? anchor.offsetLeft - 8
      : previous
        ? previous.offsetLeft + previous.offsetWidth + 8
        : 0;
    return { index, left };
  };

  const updateEdgeScroll = (clientX: number) => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const direction = columnCanvasEdgeScrollDirection({
      clientX,
      left: rect.left,
      right: rect.right,
      scrollLeft: canvas.scrollLeft,
      clientWidth: canvas.clientWidth,
      scrollWidth: canvas.scrollWidth,
    });
    if (direction === 0) stopAutoScroll();
    else startAutoScroll(direction);
  };

  return (
    <div
      ref={canvasRef}
      className='shell-column-canvas'
      aria-label={label}
      onScroll={settleMobileScroll}
      onDragStartCapture={(event: DragEvent<HTMLDivElement>) => {
        const columnId = findColumnId(event.target);
        if (!columnId || !(event.target as Element).closest('[data-column-drag-grip]')) return;
        draggingColumnIdRef.current = columnId;
      }}
      onDragOver={(event: DragEvent<HTMLDivElement>) => {
        const draggedColumnId = draggingColumnIdRef.current;
        if (!draggedColumnId) return;
        event.preventDefault();
        event.dataTransfer.dropEffect = 'move';
        setDropTarget(calculateDropTarget(event.clientX, draggedColumnId));
        updateEdgeScroll(event.clientX);
      }}
      onDrop={(event: DragEvent<HTMLDivElement>) => {
        const draggedColumnId = draggingColumnIdRef.current;
        if (!draggedColumnId || !dropTarget) return;
        event.preventDefault();
        onMoveColumn?.(draggedColumnId, dropTarget.index);
        setAnnouncement(
          `Column moved to position ${dropTarget.index + 1}.`
        );
        resetDrag();
      }}
      onDragEndCapture={resetDrag}
      onFocusCapture={(event) => activateFromEvent(event.target)}
      onPointerDown={(event) => {
        if (!isMobileViewport()) activateFromEvent(event.target);
      }}
    >
      {children}
      {dropTarget ? (
        <div
          className='shell-column-drop-indicator'
          style={{ left: `${dropTarget.left}px` }}
          role='separator'
          aria-label={`Drop Column at position ${dropTarget.index + 1}`}
        />
      ) : null}
      <span className='sr-only' aria-live='polite'>{announcement}</span>
      {columnIds.length > 1 ? (
        <nav className='shell-column-page-indicator' aria-label='Column pages'>
          <span className='shell-column-page-count' aria-live='polite'>
            {Math.max(1, columnIds.indexOf(activeColumnId) + 1)} / {columnIds.length}
          </span>
          <span className='shell-column-page-dots'>
            {columnIds.map((columnId, index) => (
              <button
                key={columnId}
                type='button'
                aria-label={`Go to Column ${index + 1} of ${columnIds.length}`}
                aria-current={columnId === activeColumnId ? 'page' : undefined}
                onClick={() => onActivateColumn(columnId, true)}
              >
                <span aria-hidden='true' />
              </button>
            ))}
          </span>
        </nav>
      ) : null}
    </div>
  );
}
