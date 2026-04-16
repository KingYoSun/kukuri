import { useEffect, useMemo, useRef, useState } from 'react';
import { createPortal } from 'react-dom';

import { cn } from '@/lib/utils';

export type ContextActionMenuItem = {
  id: string;
  label: string;
  onSelect: () => void | Promise<void>;
  tone?: 'default' | 'danger';
  disabled?: boolean;
};

export type ContextActionMenuPosition = {
  x: number;
  y: number;
};

type ContextActionMenuProps = {
  open: boolean;
  position: ContextActionMenuPosition | null;
  items: ContextActionMenuItem[];
  onClose: () => void;
};

const VIEWPORT_GUTTER_PX = 8;
const FALLBACK_MENU_WIDTH_PX = 180;
const FALLBACK_MENU_HEIGHT_PX = 120;

export function ContextActionMenu({
  open,
  position,
  items,
  onClose,
}: ContextActionMenuProps) {
  const menuRef = useRef<HTMLDivElement | null>(null);
  const [menuSize, setMenuSize] = useState({
    width: FALLBACK_MENU_WIDTH_PX,
    height: FALLBACK_MENU_HEIGHT_PX,
  });

  useEffect(() => {
    if (!open) {
      return undefined;
    }

    const handlePointerDown = (event: PointerEvent) => {
      if (menuRef.current?.contains(event.target as Node)) {
        return;
      }
      onClose();
    };

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        onClose();
      }
    };

    window.addEventListener('pointerdown', handlePointerDown);
    window.addEventListener('keydown', handleKeyDown);
    window.addEventListener('resize', onClose);

    return () => {
      window.removeEventListener('pointerdown', handlePointerDown);
      window.removeEventListener('keydown', handleKeyDown);
      window.removeEventListener('resize', onClose);
    };
  }, [onClose, open]);

  useEffect(() => {
    if (!open || !menuRef.current) {
      return;
    }
    const rect = menuRef.current.getBoundingClientRect();
    setMenuSize({
      width: rect.width || FALLBACK_MENU_WIDTH_PX,
      height: rect.height || FALLBACK_MENU_HEIGHT_PX,
    });
    menuRef.current.focus();
  }, [items.length, open, position?.x, position?.y]);

  const menuStyle = useMemo(() => {
    if (!position || typeof window === 'undefined') {
      return undefined;
    }
    const left = Math.max(
      VIEWPORT_GUTTER_PX,
      Math.min(position.x, window.innerWidth - menuSize.width - VIEWPORT_GUTTER_PX)
    );
    const top = Math.max(
      VIEWPORT_GUTTER_PX,
      Math.min(position.y, window.innerHeight - menuSize.height - VIEWPORT_GUTTER_PX)
    );
    return {
      left,
      top,
    };
  }, [menuSize.height, menuSize.width, position]);

  if (!open || !position || typeof document === 'undefined') {
    return null;
  }

  return createPortal(
    <div
      ref={menuRef}
      role='menu'
      tabIndex={-1}
      className='context-action-menu panel'
      style={menuStyle}
      onContextMenu={(event) => event.preventDefault()}
    >
      {items.map((item) => (
        <button
          key={item.id}
          type='button'
          role='menuitem'
          disabled={item.disabled}
          className={cn(
            'context-action-menu-item',
            item.tone === 'danger' && 'context-action-menu-item-danger'
          )}
          onClick={async () => {
            await item.onSelect();
            onClose();
          }}
        >
          {item.label}
        </button>
      ))}
    </div>,
    document.body
  );
}
