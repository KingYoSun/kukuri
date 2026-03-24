import * as React from 'react';

import { X } from 'lucide-react';

import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
import { cn } from '@/lib/utils';

import { type ContextPaneMode } from './types';

type ContextTab = {
  id: ContextPaneMode;
  label: string;
  summary: string;
  content: React.ReactNode;
};

type ContextPaneProps = {
  paneId: string;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  activeMode: ContextPaneMode;
  onModeChange: (mode: ContextPaneMode) => void;
  tabs: ContextTab[];
};

export function ContextPane({
  paneId,
  open,
  onOpenChange,
  activeMode,
  onModeChange,
  tabs,
}: ContextPaneProps) {
  const activeTab = tabs.find((tab) => tab.id === activeMode) ?? tabs[0];

  return (
    <>
      <div
        className='shell-overlay-backdrop shell-context-backdrop'
        data-open={open}
        onClick={() => onOpenChange(false)}
        aria-hidden='true'
      />
      <Card
        as='aside'
        id={paneId}
        className='shell-context'
        data-open={open}
        aria-label='Context pane'
      >
        <div className='shell-pane-header'>
          <div>
            <p className='eyebrow'>Context</p>
            <h2 className='shell-pane-heading'>{activeTab.label}</h2>
            <p className='shell-pane-copy'>{activeTab.summary}</p>
          </div>
          <Button
            className='shell-context-close'
            variant='ghost'
            size='icon'
            type='button'
            aria-label='Close context pane'
            onClick={() => onOpenChange(false)}
          >
            <X className='size-4' aria-hidden='true' />
          </Button>
        </div>

        <div className='shell-tab-list' role='tablist' aria-label='Context tabs'>
          {tabs.map((tab) => (
            <button
              key={tab.id}
              className={cn('shell-tab', activeMode === tab.id && 'shell-tab-active')}
              id={`${paneId}-tab-${tab.id}`}
              role='tab'
              type='button'
              aria-selected={activeMode === tab.id}
              aria-controls={`${paneId}-panel-${tab.id}`}
              tabIndex={activeMode === tab.id ? 0 : -1}
              onClick={() => onModeChange(tab.id)}
            >
              {tab.label}
            </button>
          ))}
        </div>

        {tabs.map((tab) => (
          <section
            key={tab.id}
            id={`${paneId}-panel-${tab.id}`}
            className='shell-context-panel'
            role='tabpanel'
            aria-labelledby={`${paneId}-tab-${tab.id}`}
            hidden={activeMode !== tab.id}
          >
            {tab.content}
          </section>
        ))}
      </Card>
    </>
  );
}
