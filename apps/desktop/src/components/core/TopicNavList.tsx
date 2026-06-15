import { useTranslation } from 'react-i18next';
import { Link2, Plug, Settings, SquareArrowRightExit } from 'lucide-react';

import { type TopicDiagnosticSummary } from './types';
import { cn } from '@/lib/utils';

type TopicNavListProps = {
  items: TopicDiagnosticSummary[];
  onSelectTopic: (topic: string) => void;
  onSelectChannel: (topic: string, channelId: string) => void;
  onOpenChannelSettings?: (topic: string, channelId: string) => void;
  onLeaveChannel?: (topic: string, channelId: string) => void;
  onRemoveTopic: (topic: string) => void;
  onCopyTopicLink?: (topic: string) => void;
  onToggleTopicGossip?: (topic: string, enabled: boolean) => void;
  onToggleChannelGossip?: (topic: string, channelId: string, enabled: boolean) => void;
};

export function TopicNavList({
  items,
  onSelectTopic,
  onSelectChannel,
  onOpenChannelSettings,
  onLeaveChannel,
  onRemoveTopic,
  onCopyTopicLink,
  onToggleTopicGossip,
  onToggleChannelGossip,
}: TopicNavListProps) {
  const { t } = useTranslation(['channels', 'common', 'shell']);

  return (
    <ul>
      {items.map((item) => {
        const hasChannels = Boolean(item.channels?.length);
        const publicActive = item.publicActive ?? !item.channels?.some((channel) => channel.active);
        const topicGossipJoined = item.gossipJoined ?? true;

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

            <div className='topic-actions'>
              {onToggleTopicGossip ? (
                <button
                  className={cn('topic-plug', topicGossipJoined && 'topic-plug-active')}
                  type='button'
                  aria-pressed={topicGossipJoined}
                  aria-label={t(
                    topicGossipJoined
                      ? 'shell:navigation.disconnectTopic'
                      : 'shell:navigation.connectTopic',
                    { topic: item.topic }
                  )}
                  onClick={() => onToggleTopicGossip(item.topic, !topicGossipJoined)}
                >
                  <Plug className='size-4' aria-hidden='true' />
                </button>
              ) : null}

              {onCopyTopicLink ? (
                <button
                  className='topic-copy'
                  type='button'
                  aria-label={t('common:actions.copyLink')}
                  onClick={() => onCopyTopicLink(item.topic)}
                >
                  <Link2 className='size-4' aria-hidden='true' />
                </button>
              ) : null}

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
            </div>

            <div className='topic-diagnostic'>
              <span>
                {t('shell:navigation.topicSummary', {
                  status:
                    item.connectionLabel === 'joined'
                      ? t('common:states.joined')
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
                      {item.channels?.map((channel) => {
                        const channelGossipJoined = channel.gossipJoined ?? true;
                        return (
                        <li key={channel.channelId} className='topic-subitem-row'>
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
                          {onToggleChannelGossip ? (
                            <button
                              className={cn(
                                'topic-channel-settings',
                                channelGossipJoined && 'topic-plug-active'
                              )}
                              type='button'
                              aria-pressed={channelGossipJoined}
                              aria-label={t(
                                channelGossipJoined
                                  ? 'shell:navigation.disconnectChannel'
                                  : 'shell:navigation.connectChannel',
                                { channel: channel.label }
                              )}
                              onClick={() =>
                                onToggleChannelGossip(
                                  item.topic,
                                  channel.channelId,
                                  !channelGossipJoined
                                )
                              }
                            >
                              <Plug className='size-4' aria-hidden='true' />
                            </button>
                          ) : null}
                          {onOpenChannelSettings ? (
                            <button
                              className='topic-channel-settings'
                              type='button'
                              aria-label={t('channels:actions.openSettings', {
                                channel: channel.label,
                              })}
                              onClick={() => onOpenChannelSettings(item.topic, channel.channelId)}
                            >
                              <Settings className='size-4' aria-hidden='true' />
                            </button>
                          ) : null}
                          {onLeaveChannel ? (
                            <button
                              className='topic-channel-settings'
                              type='button'
                              aria-label={t('channels:actions.leaveChannel', {
                                channel: channel.label,
                              })}
                              onClick={() => onLeaveChannel(item.topic, channel.channelId)}
                            >
                              <SquareArrowRightExit className='size-4' aria-hidden='true' />
                            </button>
                          ) : null}
                        </li>
                        );
                      })}
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
