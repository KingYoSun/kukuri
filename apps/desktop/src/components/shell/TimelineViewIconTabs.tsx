import { Bookmark, List } from 'lucide-react';
import { useRef, type KeyboardEvent } from 'react';
import { useTranslation } from 'react-i18next';

import { Button } from '@/components/ui/button';
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip';

export type TimelineViewId = 'feed' | 'bookmarks';

type TimelineViewIconTabsProps = {
  activeView: TimelineViewId;
  items: Array<{ id: TimelineViewId; label: string }>;
  onSelect: (view: TimelineViewId) => void;
};

export function TimelineViewIconTabs({
  activeView,
  items,
  onSelect,
}: TimelineViewIconTabsProps) {
  const { t } = useTranslation('shell');
  const tabRefs = useRef<Array<HTMLButtonElement | null>>([]);

  const moveSelection = (event: KeyboardEvent<HTMLButtonElement>, index: number) => {
    let nextIndex: number | undefined;
    if (event.key === 'ArrowRight' || event.key === 'ArrowDown') {
      nextIndex = (index + 1) % items.length;
    } else if (event.key === 'ArrowLeft' || event.key === 'ArrowUp') {
      nextIndex = (index - 1 + items.length) % items.length;
    } else if (event.key === 'Home') {
      nextIndex = 0;
    } else if (event.key === 'End') {
      nextIndex = items.length - 1;
    }

    if (nextIndex === undefined) return;
    event.preventDefault();
    onSelect(items[nextIndex].id);
    tabRefs.current[nextIndex]?.focus();
  };

  return (
    <TooltipProvider delayDuration={180}>
      <div
        className='shell-column-view-tabs'
        role='tablist'
        aria-label={t('workspace.timelineViews')}
      >
        {items.map((item, index) => {
          const active = activeView === item.id;
          return (
            <Tooltip key={item.id}>
              <TooltipTrigger asChild>
                <Button
                  variant={active ? 'secondary' : 'ghost'}
                  size='icon'
                  className='shell-column-view-tab'
                  role='tab'
                  type='button'
                  aria-label={item.label}
                  aria-selected={active}
                  tabIndex={active ? 0 : -1}
                  ref={(node) => {
                    tabRefs.current[index] = node;
                  }}
                  onClick={() => onSelect(item.id)}
                  onKeyDown={(event) => moveSelection(event, index)}
                >
                  {item.id === 'feed' ? (
                    <List className='size-4' aria-hidden='true' />
                  ) : (
                    <Bookmark className='size-4' aria-hidden='true' />
                  )}
                </Button>
              </TooltipTrigger>
              <TooltipContent>{item.label}</TooltipContent>
            </Tooltip>
          );
        })}
      </div>
    </TooltipProvider>
  );
}
