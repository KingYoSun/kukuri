import { useEffect, useRef, type ReactNode } from 'react';

type ColumnCanvasProps = {
  activeColumnId: string;
  children: ReactNode;
  label?: string;
  onActivateColumn: (columnId: string) => void;
};

function findColumnId(target: EventTarget | null) {
  if (!(target instanceof Element)) return null;
  return target.closest<HTMLElement>('[data-column-id]')?.dataset.columnId ?? null;
}

export function ColumnCanvas({
  activeColumnId,
  children,
  label = 'Column workspace',
  onActivateColumn,
}: ColumnCanvasProps) {
  const canvasRef = useRef<HTMLDivElement | null>(null);
  const previousActiveColumnIdRef = useRef(activeColumnId);

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
    if (columnId && columnId !== activeColumnId) onActivateColumn(columnId);
  };

  return (
    <div
      ref={canvasRef}
      className='shell-column-canvas'
      aria-label={label}
      onFocusCapture={(event) => activateFromEvent(event.target)}
      onPointerDown={(event) => activateFromEvent(event.target)}
    >
      {children}
    </div>
  );
}
