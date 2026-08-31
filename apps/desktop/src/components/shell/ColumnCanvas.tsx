import {
  Children,
  useCallback,
  useEffect,
  useRef,
  useState,
  type ReactNode,
} from 'react';
import { useTranslation } from 'react-i18next';
import { IconButtonTooltip } from '@/components/ui/icon-button';
import { prefersReducedMotion } from '@/lib/reducedMotion';
import { columnCanvasEdgeScrollDirection } from './columnCanvasGeometry';
import { nearestColumnToViewportCenter } from './columnPagingGeometry';
import { columnSwipeTargetIndex } from './columnSwipeGesture';

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
const MOBILE_SWIPE_EDGE_PX = 24;
const POINTER_DRAG_THRESHOLD_PX = 6;

type PointerDragState = {
  columnId: string;
  pointerId: number;
  startX: number;
  startY: number;
  surface: HTMLElement;
  dragging: boolean;
};

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
  label,
  onActivateColumn,
  onMoveColumn,
  onVisibleColumnIdsChange,
}: ColumnCanvasProps) {
  const { t } = useTranslation('shell');
  const canvasLabel = label ?? t('workspace.columnCanvas');
  const canvasRef = useRef<HTMLDivElement | null>(null);
  const previousActiveColumnIdRef = useRef<string | null>(null);
  const autoScrollFrameRef = useRef<number | null>(null);
  const autoScrollDirectionRef = useRef<-1 | 0 | 1>(0);
  const pointerDragRef = useRef<PointerDragState | null>(null);
  const swipeGestureRef = useRef<{
    pointerId: number;
    startX: number;
    startY: number;
  } | null>(null);
  const swipeConsumedRef = useRef(false);
  const scrollSettleTimeoutRef = useRef<number | null>(null);
  const programmaticScrollTargetRef = useRef<string | null>(null);
  const [dropTarget, setDropTarget] = useState<{ index: number; left: number } | null>(null);
  const dropTargetRef = useRef<{ index: number; left: number } | null>(null);
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

  const resetDrag = useCallback((releaseCapture = true) => {
    stopAutoScroll();
    const pointerDrag = pointerDragRef.current;
    const canvas = canvasRef.current;
    if (
      releaseCapture &&
      pointerDrag &&
      canvas &&
      typeof canvas.hasPointerCapture === 'function' &&
      canvas.hasPointerCapture(pointerDrag.pointerId)
    ) {
      canvas.releasePointerCapture(pointerDrag.pointerId);
    }
    pointerDrag?.surface.removeAttribute('data-dragging');
    pointerDragRef.current = null;
    dropTargetRef.current = null;
    setDropTarget(null);
  }, [stopAutoScroll]);

  useEffect(() => resetDrag, [resetDrag]);

  useEffect(() => () => {
    if (scrollSettleTimeoutRef.current !== null) {
      window.clearTimeout(scrollSettleTimeoutRef.current);
    }
  }, []);

  useEffect(() => {
    const cancelWithEscape = (event: KeyboardEvent) => {
      if (event.key !== 'Escape' || !pointerDragRef.current) return;
      event.preventDefault();
      resetDrag();
    };
    window.addEventListener('keydown', cancelWithEscape);
    return () => window.removeEventListener('keydown', cancelWithEscape);
  }, [resetDrag]);

  useEffect(() => {
    if (previousActiveColumnIdRef.current === activeColumnId) return;
    previousActiveColumnIdRef.current = activeColumnId;
    const column = canvasRef.current?.querySelector<HTMLElement>(
      `[data-column-id="${CSS.escape(activeColumnId)}"]`
    );
    if (typeof column?.scrollIntoView === 'function') {
      const mobile = isMobileViewport();
      const reducedMotion = prefersReducedMotion();
      if (mobile) {
        if (scrollSettleTimeoutRef.current !== null) {
          window.clearTimeout(scrollSettleTimeoutRef.current);
          scrollSettleTimeoutRef.current = null;
        }
        // route 起因の activate による smooth scroll が停滞しても、settle が途中位置の
        // Column を user scroll と誤認して activate + route 同期しないよう対象を記録する
        // (解除は目的地到達の settle か pointer / wheel のユーザー入力)。
        programmaticScrollTargetRef.current = activeColumnId;
      }
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
      const programmaticTarget = programmaticScrollTargetRef.current;
      if (programmaticTarget) {
        if (nearest === programmaticTarget) programmaticScrollTargetRef.current = null;
        return;
      }
      if (nearest && nearest !== activeColumnId) onActivateColumn(nearest, true);
    }, SCROLL_SETTLE_MS);
  };

  const activateFromEvent = (target: EventTarget | null) => {
    if (
      target instanceof Element &&
      target.closest('[data-column-gesture-owner], [data-column-preserve-activation]')
    ) {
      return;
    }
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
      aria-label={canvasLabel}
      onScroll={settleMobileScroll}
      onWheelCapture={() => {
        // wheel 入力はユーザーの scroll 引き継ぎ。programmatic scroll ガードを解除する。
        programmaticScrollTargetRef.current = null;
      }}
      onClickCapture={(event) => {
        if (!swipeConsumedRef.current) return;
        event.preventDefault();
        event.stopPropagation();
        swipeConsumedRef.current = false;
      }}
      onPointerDownCapture={(event) => {
        programmaticScrollTargetRef.current = null;
        const mobile = isMobileViewport();
        const target = event.target instanceof Element ? event.target : null;
        if (!mobile) {
          const grip = target?.closest<HTMLElement>('[data-column-drag-grip]');
          const surface = grip?.closest<HTMLElement>('[data-column-id]');
          const columnId = surface?.dataset.columnId;
          if (
            grip &&
            surface &&
            columnId &&
            event.isPrimary &&
            event.button === 0 &&
            onMoveColumn
          ) {
            pointerDragRef.current = {
              columnId,
              pointerId: event.pointerId,
              startX: event.clientX,
              startY: event.clientY,
              surface,
              dragging: false,
            };
            event.currentTarget.setPointerCapture(event.pointerId);
          }
          return;
        }
        if (columnIds.length < 2) return;
        if (swipeGestureRef.current) return;
        const canvas = canvasRef.current;
        if (!canvas) return;
        const rect = canvas.getBoundingClientRect();
        const startsAtEdge =
          event.clientX - rect.left <= MOBILE_SWIPE_EDGE_PX ||
          rect.right - event.clientX <= MOBILE_SWIPE_EDGE_PX;
        const startsOnIndicator = Boolean(target?.closest('[data-column-swipe-indicator]'));
        if (!startsAtEdge && !startsOnIndicator) return;
        swipeGestureRef.current = {
          pointerId: event.pointerId,
          startX: event.clientX,
          startY: event.clientY,
        };
      }}
      onPointerMove={(event) => {
        const pointerDrag = pointerDragRef.current;
        if (!pointerDrag || pointerDrag.pointerId !== event.pointerId) return;
        if (!pointerDrag.dragging) {
          const distance = Math.hypot(
            event.clientX - pointerDrag.startX,
            event.clientY - pointerDrag.startY
          );
          if (distance < POINTER_DRAG_THRESHOLD_PX) return;
          pointerDrag.dragging = true;
          pointerDrag.surface.setAttribute('data-dragging', 'true');
        }
        event.preventDefault();
        const nextDropTarget = calculateDropTarget(event.clientX, pointerDrag.columnId);
        dropTargetRef.current = nextDropTarget;
        setDropTarget(nextDropTarget);
        updateEdgeScroll(event.clientX);
      }}
      onPointerUpCapture={(event) => {
        const pointerDrag = pointerDragRef.current;
        if (pointerDrag && pointerDrag.pointerId === event.pointerId) {
          if (pointerDrag.dragging && dropTargetRef.current) {
            onMoveColumn?.(pointerDrag.columnId, dropTargetRef.current.index);
            setAnnouncement(
              t('workspace.columnMoved', { position: dropTargetRef.current.index + 1 })
            );
            event.preventDefault();
          }
          resetDrag();
          return;
        }
        const gesture = swipeGestureRef.current;
        if (!gesture || gesture.pointerId !== event.pointerId) return;
        swipeGestureRef.current = null;
        const targetIndex = columnSwipeTargetIndex({
          activeIndex: columnIds.indexOf(activeColumnId),
          columnCount: columnIds.length,
          deltaX: event.clientX - gesture.startX,
          deltaY: event.clientY - gesture.startY,
        });
        if (targetIndex === null) return;
        event.preventDefault();
        swipeConsumedRef.current = true;
        window.setTimeout(() => {
          swipeConsumedRef.current = false;
        }, 0);
        programmaticScrollTargetRef.current = columnIds[targetIndex];
        onActivateColumn(columnIds[targetIndex], true);
      }}
      onPointerCancelCapture={() => {
        if (pointerDragRef.current) resetDrag();
        swipeGestureRef.current = null;
      }}
      onLostPointerCapture={(event) => {
        if (pointerDragRef.current?.pointerId === event.pointerId) resetDrag(false);
      }}
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
          aria-label={t('workspace.columnDrop', { position: dropTarget.index + 1 })}
        />
      ) : null}
      <span className='sr-only' aria-live='polite'>{announcement}</span>
      {columnIds.length > 1 ? (
        <nav
          className='shell-column-page-indicator'
          aria-label={t('workspace.columnPages')}
          data-column-swipe-indicator
        >
          <span className='shell-column-page-count' aria-live='polite'>
            {Math.max(1, columnIds.indexOf(activeColumnId) + 1)} / {columnIds.length}
          </span>
          <span className='shell-column-page-dots'>
            {columnIds.map((columnId, index) => (
              <IconButtonTooltip
                key={columnId}
                label={t('workspace.columnPage', {
                  position: index + 1,
                  total: columnIds.length,
                })}
              >
                <button
                  type='button'
                  aria-label={t('workspace.columnPage', {
                    position: index + 1,
                    total: columnIds.length,
                  })}
                  aria-current={columnId === activeColumnId ? 'page' : undefined}
                  onClick={() => {
                    programmaticScrollTargetRef.current = columnId;
                    if (scrollSettleTimeoutRef.current !== null) {
                      window.clearTimeout(scrollSettleTimeoutRef.current);
                      scrollSettleTimeoutRef.current = null;
                    }
                    onActivateColumn(columnId, true);
                  }}
                >
                  <span aria-hidden='true' />
                </button>
              </IconButtonTooltip>
            ))}
          </span>
        </nav>
      ) : null}
    </div>
  );
}
