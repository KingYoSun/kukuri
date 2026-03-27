import { useTranslation } from 'react-i18next';

import { type TopicDiagnosticSummary } from './types';

type TopicNavListProps = {
  items: TopicDiagnosticSummary[];
  onSelectTopic: (topic: string) => void;
  onRemoveTopic: (topic: string) => void;
};

export function TopicNavList({ items, onSelectTopic, onRemoveTopic }: TopicNavListProps) {
  const { t } = useTranslation(['common', 'shell']);

  return (
    <ul>
      {items.map((item) => (
        <li key={item.topic} className={item.active ? 'topic-item topic-item-active' : 'topic-item'}>
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
        </li>
      ))}
    </ul>
  );
}
