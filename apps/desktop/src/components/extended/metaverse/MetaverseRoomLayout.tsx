import { useEffect, useId, useRef, useState, type ReactNode, type RefObject } from 'react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import { useColumnFullscreen } from '@/shell/ColumnPresentationContext';

export function MetaverseRoomLayout({ admitted, panelRef, before, after, children }: {
  admitted: boolean;
  panelRef: RefObject<HTMLDivElement | null>;
  before: ReactNode;
  after: ReactNode;
  children: ReactNode;
}) {
  const fullscreen = useColumnFullscreen();
  const immersive = fullscreen && admitted;
  const [open, setOpen] = useState(false);
  const toggleRef = useRef<HTMLButtonElement>(null);
  const id = useId();
  const { t } = useTranslation('metaverse');
  useEffect(() => {
    if (immersive && panelRef.current) {
      const body = panelRef.current.closest('.shell-column-body');
      if (body) body.scrollTop = 0;
    }
  }, [immersive, panelRef]);
  return (
    <div className='metaverse-panel' ref={panelRef} data-immersive={immersive || undefined}
      data-tools-open={immersive && open || undefined}>
      {immersive && <Button ref={toggleRef} className='metaverse-aux-toggle' variant='secondary'
        aria-expanded={open} aria-controls={`${id}-before ${id}-after`} onClick={() => setOpen(!open)}>
        {t(open ? 'fullscreen.closeTools' : 'fullscreen.openTools')}
      </Button>}
      <div id={`${id}-before`} className='metaverse-auxiliary metaverse-aux-before' hidden={immersive && !open}>
        {immersive && <Button variant='secondary' onClick={() => { setOpen(false); toggleRef.current?.focus(); }}>
          {t('fullscreen.closeTools')}
        </Button>}
        {before}
      </div>
      {children}
      <div id={`${id}-after`} className='metaverse-auxiliary metaverse-aux-after' hidden={immersive && !open}>
        {after}
      </div>
    </div>
  );
}
