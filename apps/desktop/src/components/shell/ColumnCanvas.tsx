import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type DragEvent,
  type ReactNode,
} from 'react';
import { columnCanvasEdgeScrollDirection } from './columnCanvasGeometry';

type ColumnCanvasProps = {
  activeColumnId: string;
  children: ReactNode;
  label?: string;
  onActivateColumn: (columnId: string, syncRoute: boolean) => void;
  onMoveColumn?: (columnId: string, targetIndex: number) => void;
};

const EDGE_SCROLL_STEP_PX = 18;

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
  label = 'Column workspace',
  onActivateColumn,
  onMoveColumn,
}: ColumnCanvasProps) {
  const canvasRef = useRef<HTMLDivElement | null>(null);
  const previousActiveColumnIdRef = useRef<string | null>(null);
  const autoScrollFrameRef = useRef<number | null>(null);
  const autoScrollDirectionRef = useRef<-1 | 0 | 1>(0);
  const draggingColumnIdRef = useRef<string | null>(null);
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
    if (window.matchMedia?.('(prefers-reduced-motion: reduce)').matches) {
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

  useEffect(() => {
    if (previousActiveColumnIdRef.current === activeColumnId) return;
    previousActiveColumnIdRef.current = activeColumnId;
    const column = canvasRef.current?.querySelector<HTMLElement>(
      `[data-column-id="${CSS.escape(activeColumnId)}"]`
    );
    if (typeof column?.scrollIntoView === 'function') {
      column.scrollIntoView({ block: 'nearest', inline: 'nearest' });
    }
  }, [activeColumnId]);

  const activateFromEvent = (target: EventTarget | null) => {
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
      onPointerDown={(event) => activateFromEvent(event.target)}
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
    </div>
  );
}
