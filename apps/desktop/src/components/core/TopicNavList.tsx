import { useTranslation } from 'react-i18next';

import { type TopicDiagnosticSummary } from './types';
import { cn } from '@/lib/utils';

type TopicNavListProps = {
  items: TopicDiagnosticSummary[];
  onSelectTopic: (topic: string) => void;
  onSelectChannel: (topic: string, channelId: string) => void;
  onRemoveTopic: (topic: string) => void;
};

export function TopicNavList({
  items,
  onSelectTopic,
  onSelectChannel,
  onRemoveTopic,
}: TopicNavListProps) {
  const { t } = useTranslation(['common', 'shell']);

  return (
    <ul>
      {items.map((item) => {
        const hasChannels = Boolean(item.channels?.length);
        const publicActive = item.publicActive ?? !item.channels?.some((channel) => channel.active);

        return (
          <li
            key={item.topic}
            className={item.active ? 'topic-item topic-item-active' : 'topic-item'}
          >
            <button className='topic-link' type='button' onClick={() => onSelectTopic(item.topic)}>
              <span className='shell-topic-link-label' title={item.topic}>
                {item.topic}
              </span>
            </button>

            {item.removable ? (
              <button
                className='topic-remove'
                type='button'
                aria-label={t('shell:navigation.removeTopic', { topic: item.topic })}
                onClick={() => onRemoveTopic(item.topic)}
              >
                x
              </button>
            ) : null}

            <div className='topic-diagnostic'>
              <span>
                {t('shell:navigation.topicSummary', {
                  status:
                    item.connectionLabel === 'joined'
                      ? t('common:states.joined')
                      : item.connectionLabel === 'relay-assisted'
                        ? t('common:states.relayAssisted')
                        : item.connectionLabel === 'idle'
                          ? t('common:states.idle')
                          : item.connectionLabel,
                  count: item.peerCount,
                })}
              </span>
              <small>{item.lastReceivedLabel}</small>
            </div>

            {item.active ? (
              <div className='topic-scope-group'>
                <button
                  className={cn('topic-subitem', publicActive && 'topic-subitem-active')}
                  type='button'
                  aria-pressed={publicActive}
                  onClick={() => onSelectTopic(item.topic)}
                >
                  <span className='shell-topic-link-label'>{t('common:audience.public')}</span>
                  <small>{t('shell:navigation.publicScope')}</small>
                </button>

                {hasChannels ? (
                  <>
                    <div className='topic-subsection-label'>{t('shell:navigation.channelsGroup')}</div>
                    <ul className='topic-sublist'>
                      {item.channels?.map((channel) => (
                        <li key={channel.channelId}>
                          <button
                            className={cn(
                              'topic-subitem',
                              channel.active && 'topic-subitem-active'
                            )}
                            type='button'
                            aria-pressed={channel.active}
                            onClick={() => onSelectChannel(item.topic, channel.channelId)}
                          >
                            <span className='shell-topic-link-label'>{channel.label}</span>
                            <small>{t(`channels:audienceOptions.${channel.audienceKind}`)}</small>
                          </button>
                        </li>
                      ))}
                    </ul>
                  </>
                ) : null}
              </div>
            ) : null}
          </li>
        );
      })}
    </ul>
  );
}
