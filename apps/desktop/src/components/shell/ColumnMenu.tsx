import { useEffect, useId, useRef, useState, type FocusEvent, type KeyboardEvent } from 'react';
import { createPortal } from 'react-dom';
import { useTranslation } from 'react-i18next';
import {
  ChevronLeft,
  ChevronRight,
  Maximize2,
  Minimize2,
  MoreHorizontal,
  Pin,
  PinOff,
  X,
} from 'lucide-react';

import { Button } from '@/components/ui/button';
import type { ColumnSpan } from '@/shell/slices/workspace';

type ColumnMenuProps = {
  onClose?: () => void;
  onMoveLeft?: () => void;
  onMoveRight?: () => void;
  onPinnedChange?: (pinned: boolean) => void;
  onSpanChange?: (span: ColumnSpan) => void;
  onToggleFullscreen?: () => void;
  fullscreen?: boolean;
  pinned: boolean;
  span: ColumnSpan;
  spanOptions: ColumnSpan[];
  title: string;
};

type MenuFocusTarget = 'first' | 'last';
// 'opening' は open 直後(利用者の scroll 操作がまだ無い)状態。focus 移動や直前の click に伴う
// 遅延 scroll イベントを「利用者の scroll」と誤認して閉じないために区別する。
type MenuInteraction = 'opening' | 'keyboard' | 'pointer';

const MENU_ITEM_SELECTOR = '[role^="menuitem"]:not(:disabled)';

export function ColumnMenu({
  onClose,
  onMoveLeft,
  onMoveRight,
  onPinnedChange,
  onSpanChange,
  onToggleFullscreen,
  fullscreen = false,
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
  // open 直後に focus を移す先。menu が DOM に入った後の effect で消費する。
  const pendingFocusRef = useRef<MenuFocusTarget | null>(null);
  // 直近の操作手段。open 直後と keyboard 操作中は scroll 由来の close を抑止し、placement だけ追従させる。
  const interactionRef = useRef<MenuInteraction>('opening');

  useEffect(() => {
    if (!open) return undefined;
    const onPointerDown = (event: PointerEvent) => {
      interactionRef.current = 'pointer';
      const target = event.target as Node;
      if (!menuRef.current?.contains(target) && !triggerRef.current?.contains(target)) {
        setOpen(false);
      }
    };
    const onWheel = () => {
      interactionRef.current = 'pointer';
    };
    const onTouchMove = () => {
      interactionRef.current = 'pointer';
    };
    const close = () => setOpen(false);
    // trigger の現在位置に合わせて menu を置き直す(focus 移動や layout 変化に伴う scroll への追従)。
    const followTrigger = () => {
      const trigger = triggerRef.current;
      if (!trigger) return;
      const rect = trigger.getBoundingClientRect();
      setPlacement((current) =>
        current
          ? { ...current, top: rect.bottom + 4, right: Math.max(8, window.innerWidth - rect.right) }
          : current
      );
    };
    const onScroll = (event: Event) => {
      const target = event.target instanceof Node ? event.target : null;
      // menu / trigger 自身の scroll では閉じない。
      if (target && (menuRef.current?.contains(target) || triggerRef.current?.contains(target))) {
        return;
      }
      // 利用者の wheel / touch / pointer 由来でない scroll(open 直後の遅延 scroll イベント、
      // keyboard 操作に伴う focus scroll 等)では閉じず、trigger に追従させる。
      if (interactionRef.current !== 'pointer') {
        followTrigger();
        return;
      }
      close();
    };
    window.addEventListener('pointerdown', onPointerDown);
    window.addEventListener('wheel', onWheel, { capture: true, passive: true });
    window.addEventListener('touchmove', onTouchMove, { capture: true, passive: true });
    window.addEventListener('resize', close);
    document.addEventListener('scroll', onScroll, true);
    return () => {
      window.removeEventListener('pointerdown', onPointerDown);
      window.removeEventListener('wheel', onWheel, { capture: true });
      window.removeEventListener('touchmove', onTouchMove, { capture: true });
      window.removeEventListener('resize', close);
      document.removeEventListener('scroll', onScroll, true);
    };
  }, [open]);

  // menu が描画された後に、open 時に指定された先頭 / 末尾の menuitem へ focus を移す。
  useEffect(() => {
    if (!open || !placement || !pendingFocusRef.current) return;
    const target = pendingFocusRef.current;
    pendingFocusRef.current = null;
    const items = Array.from(
      menuRef.current?.querySelectorAll<HTMLButtonElement>(MENU_ITEM_SELECTOR) ?? []
    );
    if (items.length === 0) return;
    (target === 'last' ? items[items.length - 1] : items[0]).focus();
  }, [open, placement]);

  const getItems = () =>
    Array.from(menuRef.current?.querySelectorAll<HTMLButtonElement>(MENU_ITEM_SELECTOR) ?? []);

  const openMenu = (focusTarget: MenuFocusTarget) => {
    const trigger = triggerRef.current;
    if (!trigger) return;
    // open 直後は利用者の scroll 操作が無い状態として扱い、wheel / touch / pointerdown で 'pointer' に遷移する。
    interactionRef.current = 'opening';
    pendingFocusRef.current = focusTarget;
    const rect = trigger.getBoundingClientRect();
    const fullscreenTarget = document.fullscreenElement;
    setPlacement({
      top: rect.bottom + 4,
      right: Math.max(8, window.innerWidth - rect.right),
      target:
        fullscreenTarget instanceof Element && fullscreenTarget.contains(trigger)
          ? fullscreenTarget
          : trigger.closest('.shell-phase1') ?? document.body,
    });
    setOpen(true);
  };
  const closeMenu = () => {
    pendingFocusRef.current = null;
    setOpen(false);
    setPlacement(null);
    triggerRef.current?.focus();
  };
  const select = (action?: () => void) => {
    action?.();
    closeMenu();
  };
  const focusItem = (position: MenuFocusTarget) => {
    const items = getItems();
    if (items.length === 0) return;
    (position === 'last' ? items[items.length - 1] : items[0]).focus();
  };
  const focusSibling = (direction: 1 | -1) => {
    const items = getItems();
    if (items.length === 0) return;
    const currentIndex = items.indexOf(document.activeElement as HTMLButtonElement);
    const nextIndex = currentIndex < 0
      ? 0
      : (currentIndex + direction + items.length) % items.length;
    items[nextIndex].focus();
  };

  const onTriggerKeyDown = (event: KeyboardEvent<HTMLButtonElement>) => {
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault();
      const target: MenuFocusTarget = event.key === 'ArrowDown' ? 'first' : 'last';
      if (open) {
        interactionRef.current = 'keyboard';
        focusItem(target);
      } else {
        openMenu(target);
      }
    } else if (event.key === 'Escape' && open) {
      event.preventDefault();
      closeMenu();
    }
  };

  const onMenuKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    interactionRef.current = 'keyboard';
    switch (event.key) {
      case 'Escape':
      case 'Tab':
        event.preventDefault();
        closeMenu();
        break;
      case 'ArrowDown':
        event.preventDefault();
        focusSibling(1);
        break;
      case 'ArrowUp':
        event.preventDefault();
        focusSibling(-1);
        break;
      case 'Home':
        event.preventDefault();
        focusItem('first');
        break;
      case 'End':
        event.preventDefault();
        focusItem('last');
        break;
      default:
        break;
    }
  };

  const onMenuBlur = (event: FocusEvent<HTMLDivElement>) => {
    const next = event.relatedTarget;
    // relatedTarget が無い場合(document 外へ離脱、unmount 等)は判断せず維持する。
    if (!(next instanceof Node)) return;
    if (menuRef.current?.contains(next) || triggerRef.current?.contains(next)) return;
    setOpen(false);
    setPlacement(null);
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
        onKeyDown={onTriggerKeyDown}
        onClick={() => {
          if (open) {
            closeMenu();
            return;
          }
          // keyboard の Enter / Space も click として届く(どちらも先頭 menuitem へ focus を移す)。
          openMenu('first');
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
          onKeyDown={onMenuKeyDown}
          onBlur={onMenuBlur}
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
          {onToggleFullscreen ? (
            <button type='button' role='menuitem' onClick={() => select(onToggleFullscreen)}>
              {fullscreen ? (
                <Minimize2 className='size-4' aria-hidden='true' />
              ) : (
                <Maximize2 className='size-4' aria-hidden='true' />
              )}
              {t(fullscreen ? 'columnMenu.exitFullscreen' : 'columnMenu.enterFullscreen', {
                title,
              })}
            </button>
          ) : null}
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
