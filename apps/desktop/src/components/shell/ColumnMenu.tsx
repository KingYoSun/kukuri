import { useEffect, useId, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { useTranslation } from 'react-i18next';
import { ChevronLeft, ChevronRight, MoreHorizontal, Pin, PinOff, X } from 'lucide-react';

import { Button } from '@/components/ui/button';
import type { ColumnSpan } from '@/shell/slices/workspace';

type ColumnMenuProps = {
  onClose?: () => void;
  onMoveLeft?: () => void;
  onMoveRight?: () => void;
  onPinnedChange?: (pinned: boolean) => void;
  onSpanChange?: (span: ColumnSpan) => void;
  pinned: boolean;
  span: ColumnSpan;
  spanOptions: ColumnSpan[];
  title: string;
};

export function ColumnMenu({
  onClose,
  onMoveLeft,
  onMoveRight,
  onPinnedChange,
  onSpanChange,
  pinned,
  span,
  spanOptions,
  title,
}: ColumnMenuProps) {
  const { t } = useTranslation('shell');
  const [open, setOpen] = useState(false);
  const [placement, setPlacement] = useState<{
    top: number;
    right: number;
    target: Element;
  } | null>(null);
  const menuId = useId();
  const triggerRef = useRef<HTMLButtonElement | null>(null);
  const menuRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (!open) return undefined;
    const onPointerDown = (event: PointerEvent) => {
      const target = event.target as Node;
      if (!menuRef.current?.contains(target) && !triggerRef.current?.contains(target)) {
        setOpen(false);
      }
    };
    const close = () => setOpen(false);
    window.addEventListener('pointerdown', onPointerDown);
    window.addEventListener('resize', close);
    document.addEventListener('scroll', close, true);
    return () => {
      window.removeEventListener('pointerdown', onPointerDown);
      window.removeEventListener('resize', close);
      document.removeEventListener('scroll', close, true);
    };
  }, [open]);

  const closeMenu = () => {
    setOpen(false);
    setPlacement(null);
    window.requestAnimationFrame(() => triggerRef.current?.focus());
  };
  const select = (action?: () => void) => {
    action?.();
    closeMenu();
  };
  const focusSibling = (direction: 1 | -1) => {
    const items = Array.from(
      menuRef.current?.querySelectorAll<HTMLButtonElement>('[role^="menuitem"]:not(:disabled)') ?? []
    );
    if (items.length === 0) return;
    const currentIndex = items.indexOf(document.activeElement as HTMLButtonElement);
    const nextIndex = currentIndex < 0
      ? 0
      : (currentIndex + direction + items.length) % items.length;
    items[nextIndex].focus();
  };

  return (
    <div className='shell-column-menu-wrap'>
      <Button
        ref={triggerRef}
        variant='ghost'
        size='icon'
        type='button'
        aria-controls={menuId}
        aria-expanded={open}
        aria-haspopup='menu'
        aria-label={t(open ? 'columnMenu.close' : 'columnMenu.open', { title })}
        onClick={(event) => {
          if (open) {
            closeMenu();
            return;
          }
          const rect = event.currentTarget.getBoundingClientRect();
          setPlacement({
            top: rect.bottom + 4,
            right: Math.max(8, window.innerWidth - rect.right),
            target: event.currentTarget.closest('.shell-phase1') ?? document.body,
          });
          setOpen(true);
        }}
      >
        <MoreHorizontal className='size-4' aria-hidden='true' />
      </Button>
      {open && placement && typeof document !== 'undefined' ? createPortal((
        <div
          ref={menuRef}
          id={menuId}
          className='shell-column-menu panel'
          style={{
            top: `${placement.top}px`,
            right: `${placement.right}px`,
          }}
          role='menu'
          aria-label={t('columnMenu.actions', { title })}
          onKeyDown={(event) => {
            if (event.key === 'Escape') {
              event.preventDefault();
              closeMenu();
            } else if (event.key === 'ArrowDown') {
              event.preventDefault();
              focusSibling(1);
            } else if (event.key === 'ArrowUp') {
              event.preventDefault();
              focusSibling(-1);
            }
          }}
        >
          <button
            type='button'
            role='menuitem'
            disabled={!onMoveLeft}
            onClick={() => select(onMoveLeft)}
          >
            <ChevronLeft className='size-4' aria-hidden='true' />
            {t('columnMenu.moveLeft', { title })}
          </button>
          <button
            type='button'
            role='menuitem'
            disabled={!onMoveRight}
            onClick={() => select(onMoveRight)}
          >
            <ChevronRight className='size-4' aria-hidden='true' />
            {t('columnMenu.moveRight', { title })}
          </button>
          <div className='shell-column-menu-separator' role='separator' />
          {spanOptions.map((option) => (
            <button
              key={option}
              type='button'
              role='menuitemradio'
              aria-checked={span === option}
              disabled={!onSpanChange || span === option}
              onClick={() => select(() => onSpanChange?.(option))}
            >
              {t('columnMenu.span', { count: option })}
            </button>
          ))}
          {onPinnedChange ? (
            <button
              type='button'
              role='menuitem'
              onClick={() => select(() => onPinnedChange(!pinned))}
            >
              {pinned ? (
                <PinOff className='size-4' aria-hidden='true' />
              ) : (
                <Pin className='size-4' aria-hidden='true' />
              )}
              {t(pinned ? 'columnMenu.unpin' : 'columnMenu.pin', { title })}
            </button>
          ) : null}
          {onClose ? (
            <button type='button' role='menuitem' onClick={() => select(onClose)}>
              <X className='size-4' aria-hidden='true' />
              {t('columnMenu.closeColumn', { title })}
            </button>
          ) : null}
        </div>
      ), placement.target) : null}
    </div>
  );
}
