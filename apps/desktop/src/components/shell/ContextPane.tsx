import * as React from 'react';

import { X } from 'lucide-react';
import { useTranslation } from 'react-i18next';

import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';

type ContextPaneProps = {
  paneId: string;
  title: string;
  summary?: string | null;
  showBackdrop?: boolean;
  stackIndex?: number;
  onClose: () => void;
  children: React.ReactNode;
};

const DETAIL_PANE_INDEX_VAR = '--shell-detail-pane-index' as const;

export function ContextPane({
  paneId,
  title,
  showBackdrop = false,
  stackIndex = 0,
  onClose,
  children,
}: ContextPaneProps) {
  const { t } = useTranslation('shell');
  const paneStyle = {
    [DETAIL_PANE_INDEX_VAR]: String(stackIndex),
  } as React.CSSProperties;

  return (
    <>
      <div
        className='shell-overlay-backdrop shell-context-backdrop'
        data-open={showBackdrop}
        onClick={onClose}
        aria-hidden='true'
      />
      <Card
        as='aside'
        id={paneId}
        className='shell-context shell-detail-pane'
        data-open='true'
        aria-label={title}
        style={paneStyle}
      >
        <div className='shell-pane-header shell-pane-header-compact'>
          <div>
            <p className='eyebrow'>{title}</p>
            <span id={`${paneId}-title`} className='sr-only'>
              {title}
            </span>
          </div>
          <Button
            className='shell-context-close shell-icon-button'
            variant='ghost'
            size='icon'
            type='button'
            aria-label={t('context.close', { title })}
            onClick={onClose}
          >
            <X className='size-5' aria-hidden='true' />
          </Button>
        </div>

        <div className='shell-context-panel'>{children}</div>
      </Card>
    </>
  );
}
