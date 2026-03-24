import * as React from 'react';

import { X } from 'lucide-react';

import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
import { cn } from '@/lib/utils';

import { type PrimarySection } from './types';

type PrimaryNavItem = {
  id: PrimarySection;
  label: string;
  description: string;
};

type ShellNavRailProps = {
  railId: string;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  primaryItems: PrimaryNavItem[];
  activePrimarySection: PrimarySection;
  onSelectPrimarySection: (section: PrimarySection) => void;
  addTopicControl: React.ReactNode;
  topicList: React.ReactNode;
  topicCount: number;
};

export function ShellNavRail({
  railId,
  open,
  onOpenChange,
  primaryItems,
  activePrimarySection,
  onSelectPrimarySection,
  addTopicControl,
  topicList,
  topicCount,
}: ShellNavRailProps) {
  return (
    <>
      <div
        className='shell-overlay-backdrop shell-nav-backdrop'
        data-open={open}
        onClick={() => onOpenChange(false)}
        aria-hidden='true'
      />
      <Card
        as='aside'
        tone='accent'
        id={railId}
        className='shell-nav'
        data-open={open}
        aria-label='Primary navigation'
      >
        <div className='shell-pane-header shell-pane-header-compact'>
          <div>
            <p className='eyebrow'>Navigate</p>
            <h2 className='shell-pane-heading'>Workspace</h2>
          </div>
          <Button
            className='shell-mobile-close'
            variant='ghost'
            size='icon'
            type='button'
            aria-label='Close navigation'
            onClick={() => onOpenChange(false)}
          >
            <X className='size-4' aria-hidden='true' />
          </Button>
        </div>

        <nav className='shell-primary-nav' aria-label='Primary sections'>
          {primaryItems.map((item) => (
            <button
              key={item.id}
              className={cn(
                'shell-primary-nav-item',
                activePrimarySection === item.id && 'shell-primary-nav-item-active'
              )}
              type='button'
              aria-current={activePrimarySection === item.id ? 'location' : undefined}
              onClick={() => onSelectPrimarySection(item.id)}
            >
              <span className='shell-primary-nav-label'>{item.label}</span>
              <span className='shell-primary-nav-copy'>{item.description}</span>
            </button>
          ))}
        </nav>

        <div className='shell-nav-topic-entry'>{addTopicControl}</div>

        <section className='topic-list shell-nav-topic-list'>
          <div className='panel-header'>
            <h3>Tracked Topics</h3>
            <small>{topicCount} active</small>
          </div>
          {topicList}
        </section>
      </Card>
    </>
  );
}
