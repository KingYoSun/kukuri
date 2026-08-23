import { GripVertical, Pin, PinOff, X } from 'lucide-react';
import { useEffect, useRef, useState, type DragEvent, type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';

import { Button } from '@/components/ui/button';
import type { ColumnSpan } from '@/shell/slices/workspace';
import { useColumnRuntime } from '@/shell/ColumnRuntimeContext';
import { ColumnMenu } from './ColumnMenu';

type ColumnSurfaceProps = {
  active: boolean;
  children: ReactNode;
  columnId: string;
  footer?: ReactNode;
  headerActions?: ReactNode;
  onPinnedChange?: (pinned: boolean) => void;
  onClose?: () => void;
  onMoveLeft?: () => void;
  onMoveRight?: () => void;
  onSpanChange?: (span: ColumnSpan) => void;
  pinned: boolean;
  position: number;
  scopeLabel: string;
  span: ColumnSpan;
  spanOptions?: ColumnSpan[];
  title: string;
  total: number;
  resourceManaged?: boolean;
};

export function ColumnSurface({
  active,
  children,
  columnId,
  footer,
  headerActions,
  onPinnedChange,
  onClose,
  onMoveLeft,
  onMoveRight,
  onSpanChange,
  pinned,
  position,
  scopeLabel,
  span,
  spanOptions = [span],
  title,
  total,
  resourceManaged = false,
}: ColumnSurfaceProps) {
  const { t } = useTranslation('shell');
  const runtime = useColumnRuntime();
  const surfaceRef = useRef<HTMLElement | null>(null);
  const [dragging, setDragging] = useState(false);
  const [announcement, setAnnouncement] = useState('');
  const stateLabel = pinned ? 'Pinned' : 'Temporary';
  const accessibleLabel = `${title} Column, Column ${position} of ${total}, ${span} span, ${
    active ? 'Active' : 'Inactive'
  }, ${stateLabel}`;

  useEffect(() => {
    if (!resourceManaged || (!runtime.suspended && runtime.audioFocused)) return;
    const media = surfaceRef.current?.querySelectorAll<HTMLMediaElement>('audio, video');
    media?.forEach((item) => {
      if (!item.muted && !item.paused && (runtime.suspended || !runtime.audioFocused)) {
        item.pause();
      }
    });
  }, [resourceManaged, runtime.audioFocused, runtime.suspended]);

  return (
    <section
      ref={surfaceRef}
      className='shell-column-surface'
      data-active={active || undefined}
      data-column-id={columnId}
      data-dragging={dragging || undefined}
      data-pinned={pinned || undefined}
      data-span={span}
      data-transient={!pinned || undefined}
      data-runtime-visible={runtime.visible}
      data-runtime-suspended={runtime.suspended || undefined}
      aria-current={active ? 'true' : undefined}
      aria-label={accessibleLabel}
      aria-roledescription='Column'
      tabIndex={-1}
      onPlayCapture={(event) => {
        if (event.target instanceof HTMLMediaElement && !event.target.muted) {
          runtime.requestAudioFocus();
        }
      }}
      onPauseCapture={(event) => {
        if (event.target instanceof HTMLMediaElement) {
          runtime.releaseAudioFocus();
        }
      }}
    >
      <header className='shell-column-header'>
        <Button
          variant='ghost'
          size='icon'
          type='button'
          className='shell-column-drag-grip'
          data-column-drag-grip
          draggable
          aria-label={t('columnMenu.drag', { title })}
          onDragStart={(event: DragEvent<HTMLButtonElement>) => {
            setDragging(true);
            event.dataTransfer.effectAllowed = 'move';
            event.dataTransfer.setData('text/x-kukuri-column-id', columnId);
            const surface = event.currentTarget.closest<HTMLElement>('[data-column-id]');
            if (surface) {
              const rect = surface.getBoundingClientRect();
              event.dataTransfer.setDragImage(
                surface,
                Math.max(0, event.clientX - rect.left),
                Math.max(0, event.clientY - rect.top)
              );
            }
          }}
          onDragEnd={() => setDragging(false)}
        >
          <GripVertical className='size-4' aria-hidden='true' />
        </Button>
        <div className='shell-column-heading'>
          <div className='shell-column-title-row'>
            <h2>{title}</h2>
            {active ? <span className='shell-column-state-label'>Active</span> : null}
            <span className='shell-column-state-label'>{stateLabel}</span>
          </div>
          <p>{scopeLabel}</p>
        </div>
        {headerActions || onPinnedChange || onClose ? (
          <div className='shell-column-header-actions'>
            {headerActions}
            {onPinnedChange ? (
              <Button
                variant='ghost'
                size='icon'
                type='button'
                className='shell-column-pin-button'
                aria-label={t(pinned ? 'columnMenu.unpin' : 'columnMenu.pin', { title })}
                aria-pressed={pinned}
                onClick={() => onPinnedChange(!pinned)}
              >
                {pinned ? (
                  <PinOff className='size-4' aria-hidden='true' />
                ) : (
                  <Pin className='size-4' aria-hidden='true' />
                )}
              </Button>
            ) : null}
            {onClose ? (
              <Button
                variant='ghost'
                size='icon'
                type='button'
                className='shell-column-close-button'
                aria-label={t('columnMenu.closeColumn', { title })}
                onClick={onClose}
              >
                <X className='size-4' aria-hidden='true' />
              </Button>
            ) : null}
            <ColumnMenu
              title={title}
              pinned={pinned}
              span={span}
              spanOptions={spanOptions}
              onMoveLeft={
                onMoveLeft
                  ? () => {
                      onMoveLeft();
                      setAnnouncement(
                        t('columnMenu.moved', { title, position: position - 1, total })
                      );
                    }
                  : undefined
              }
              onMoveRight={
                onMoveRight
                  ? () => {
                      onMoveRight();
                      setAnnouncement(
                        t('columnMenu.moved', { title, position: position + 1, total })
                      );
                    }
                  : undefined
              }
              onPinnedChange={onPinnedChange}
              onClose={onClose}
              onSpanChange={
                onSpanChange
                  ? (nextSpan) => {
                      onSpanChange(nextSpan);
                      setAnnouncement(t('columnMenu.spanChanged', { title, count: nextSpan }));
                    }
                  : undefined
              }
            />
          </div>
        ) : (
          <div className='shell-column-header-actions'>
            <ColumnMenu
              title={title}
              pinned={pinned}
              span={span}
              spanOptions={spanOptions}
              onMoveLeft={onMoveLeft}
              onMoveRight={onMoveRight}
              onSpanChange={onSpanChange}
            />
          </div>
        )}
      </header>
      <span className='sr-only' aria-live='polite'>{announcement}</span>
      <div className='shell-column-body'>{children}</div>
      {footer ? <footer className='shell-column-footer'>{footer}</footer> : null}
    </section>
  );
}
