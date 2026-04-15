import { useMemo, useRef } from 'react';
import { ChevronLeft, ChevronRight, X } from 'lucide-react';
import { useTranslation } from 'react-i18next';

import { Button } from '@/components/ui/button';
import { Dialog, DialogContent, DialogDescription, DialogTitle } from '@/components/ui/dialog';

type MediaViewerDialogProps = {
  items: Array<{
    hash: string;
    src: string | null;
    mime: string;
  }>;
  index: number;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onIndexChange: (index: number) => void;
};

function clampIndex(index: number, length: number) {
  if (length === 0) {
    return 0;
  }
  if (index < 0) {
    return length - 1;
  }
  if (index >= length) {
    return 0;
  }
  return index;
}

export function MediaViewerDialog({
  items,
  index,
  open,
  onOpenChange,
  onIndexChange,
}: MediaViewerDialogProps) {
  const { t } = useTranslation('common');
  const pointerStartXRef = useRef<number | null>(null);
  const currentIndex = clampIndex(index, items.length);
  const currentItem = items[currentIndex] ?? null;
  const canNavigate = items.length > 1;
  const descriptionLabel = useMemo(
    () =>
      canNavigate ? `${t('media.imageAlt')} ${currentIndex + 1} / ${items.length}` : t('media.imageAlt'),
    [canNavigate, currentIndex, items.length, t]
  );

  const moveBy = (delta: number) => {
    if (!canNavigate) {
      return;
    }
    onIndexChange(clampIndex(currentIndex + delta, items.length));
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className='media-viewer-dialog' hideClose>
        <DialogTitle className='sr-only'>{t('media.imageAlt')}</DialogTitle>
        <DialogDescription className='sr-only'>{descriptionLabel}</DialogDescription>
        <div
          className='media-viewer-body'
          tabIndex={-1}
          onKeyDown={(event) => {
            if (event.key === 'ArrowLeft') {
              event.preventDefault();
              moveBy(-1);
            }
            if (event.key === 'ArrowRight') {
              event.preventDefault();
              moveBy(1);
            }
          }}
        >
          <Button
            variant='ghost'
            size='icon'
            type='button'
            className='media-viewer-close'
            onClick={() => onOpenChange(false)}
            aria-label='Close dialog'
          >
            <X className='size-5' aria-hidden='true' />
          </Button>
          {canNavigate ? (
            <Button
              variant='secondary'
              size='icon'
              type='button'
              className='media-viewer-nav media-viewer-nav-prev'
              onClick={() => moveBy(-1)}
              aria-label='Previous image'
            >
              <ChevronLeft className='size-5' aria-hidden='true' />
            </Button>
          ) : null}
          <div
            className='media-viewer-stage'
            onPointerDown={(event) => {
              pointerStartXRef.current = event.clientX;
            }}
            onPointerUp={(event) => {
              const startX = pointerStartXRef.current;
              pointerStartXRef.current = null;
              if (startX === null) {
                return;
              }
              const deltaX = event.clientX - startX;
              if (Math.abs(deltaX) < 40) {
                return;
              }
              moveBy(deltaX > 0 ? -1 : 1);
            }}
          >
            {currentItem?.src ? (
              <img
                className='media-viewer-image'
                src={currentItem.src}
                alt={t('media.imageAlt')}
              />
            ) : (
              <div className='media-viewer-empty'>{t('media.syncingImage')}</div>
            )}
          </div>
          {canNavigate ? (
            <Button
              variant='secondary'
              size='icon'
              type='button'
              className='media-viewer-nav media-viewer-nav-next'
              onClick={() => moveBy(1)}
              aria-label='Next image'
            >
              <ChevronRight className='size-5' aria-hidden='true' />
            </Button>
          ) : null}
        </div>
      </DialogContent>
    </Dialog>
  );
}
