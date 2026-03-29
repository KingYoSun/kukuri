import * as React from 'react';

import { X } from 'lucide-react';
import { useTranslation } from 'react-i18next';

import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';

type ShellNavRailProps = {
  railId: string;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  headerContent: React.ReactNode;
  addTopicControl: React.ReactNode;
  channelSummary?: React.ReactNode;
  channelAction?: React.ReactNode;
  topicList: React.ReactNode;
  topicCount: number;
};

export function ShellNavRail({
  railId,
  open,
  onOpenChange,
  headerContent,
  addTopicControl,
  channelSummary,
  channelAction,
  topicList,
  topicCount,
}: ShellNavRailProps) {
  const { t } = useTranslation('shell');
  const hasChannelAction = channelAction !== null && channelAction !== undefined;

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
        aria-label={t('navigation.primaryNavigation')}
      >
        <div className='shell-pane-header shell-pane-header-compact'>
          <p className='eyebrow'>{t('navigation.title')}</p>
          <Button
            className='shell-mobile-close shell-icon-button'
            variant='ghost'
            size='icon'
            type='button'
            aria-label={t('navigation.close')}
            onClick={() => onOpenChange(false)}
          >
            <X className='size-5' aria-hidden='true' />
          </Button>
        </div>

        <div className='shell-nav-meta'>{headerContent}</div>

        <div className='shell-nav-topic-entry'>{addTopicControl}</div>

        {hasChannelAction ? (
          <section className='shell-nav-topic-entry shell-nav-channel-summary'>
            <div className='shell-nav-channel-copy'>
              <span className='shell-nav-accordion-title'>{t('navigation.channel')}</span>
              {channelSummary ? (
                <span className='shell-nav-accordion-summary'>{channelSummary}</span>
              ) : null}
            </div>
            {channelAction}
          </section>
        ) : null}

        <section className='topic-list shell-nav-topic-list'>
          <div className='panel-header'>
            <h3>{t('navigation.trackedTopics')}</h3>
            <small>{t('navigation.activeCount', { count: topicCount })}</small>
          </div>
          {topicList}
        </section>
      </Card>
    </>
  );
}
