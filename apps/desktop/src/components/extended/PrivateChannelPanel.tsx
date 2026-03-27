import type { FormEventHandler } from 'react';
import { useTranslation } from 'react-i18next';

import { Button } from '@/components/ui/button';
import { Card, CardHeader } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Notice } from '@/components/ui/notice';
import { Select } from '@/components/ui/select';
import { Textarea } from '@/components/ui/textarea';
import { cn } from '@/lib/utils';

import {
  type ChannelAudienceOption,
  type ExtendedPanelStatus,
  type InviteOutputLabel,
  type PrivateChannelListItemView,
  type PrivateChannelPendingAction,
} from './types';

function audienceSummaryLabel(
  label: InviteOutputLabel,
  t: ReturnType<typeof useTranslation<'channels'>>['t']
): string {
  if (label === 'grant') {
    return t('latestGrant');
  }
  if (label === 'share') {
    return t('latestShare');
  }
  return t('latestInvite');
}

function shortPubkey(pubkey: string): string {
  return pubkey.slice(0, 12);
}

function policyDescription(
  audienceKind: PrivateChannelListItemView['channel']['audience_kind'],
  t: ReturnType<typeof useTranslation<'channels'>>['t']
) {
  if (audienceKind === 'friend_only') {
    return t('policies.friend_only');
  }
  if (audienceKind === 'friend_plus') {
    return t('policies.friend_plus');
  }
  return t('policies.invite_only');
}

type PrivateChannelPanelProps = {
  status: ExtendedPanelStatus;
  error: string | null;
  pendingAction: PrivateChannelPendingAction;
  channelLabel: string;
  channelAudience: ChannelAudienceOption['value'];
  channelAudienceOptions: ChannelAudienceOption[];
  inviteTokenInput: string;
  inviteOutput: string | null;
  inviteOutputLabel: InviteOutputLabel;
  channels: PrivateChannelListItemView[];
  selectedChannel: PrivateChannelListItemView['channel'] | null;
  onChannelLabelChange: (value: string) => void;
  onChannelAudienceChange: (value: ChannelAudienceOption['value']) => void;
  onInviteTokenChange: (value: string) => void;
  onCreateChannel: FormEventHandler<HTMLFormElement>;
  onJoinInvite: FormEventHandler<HTMLFormElement>;
  onJoinGrant: () => void;
  onJoinShare: () => void;
  onSelectChannel: (channelId: string) => void;
  onCreateInvite: () => void;
  onCreateGrant: () => void;
  onCreateShare: () => void;
  onFreeze: () => void;
  onRotate: () => void;
};

export function PrivateChannelPanel({
  status,
  error,
  pendingAction,
  channelLabel,
  channelAudience,
  channelAudienceOptions,
  inviteTokenInput,
  inviteOutput,
  inviteOutputLabel,
  channels,
  selectedChannel,
  onChannelLabelChange,
  onChannelAudienceChange,
  onInviteTokenChange,
  onCreateChannel,
  onJoinInvite,
  onJoinGrant,
  onJoinShare,
  onSelectChannel,
  onCreateInvite,
  onCreateGrant,
  onCreateShare,
  onFreeze,
  onRotate,
}: PrivateChannelPanelProps) {
  const { t } = useTranslation(['channels', 'common']);
  const channelActionDisabled = pendingAction !== null;
  const selectedChannelId = selectedChannel?.channel_id ?? null;

  return (
    <Card className='panel-subsection'>
      <CardHeader>
        <h3>{t('channels:title')}</h3>
        <small>{t('channels:joined', { count: channels.length })}</small>
      </CardHeader>

      {status === 'loading' ? <Notice>{t('channels:loading')}</Notice> : null}
      {status === 'error' && error ? <Notice tone='destructive'>{error}</Notice> : null}

      <div className='extended-module-stack'>
        <form className='composer composer-compact' onSubmit={onCreateChannel}>
          <Label>
            <span>{t('channels:editor.createChannel')}</span>
            <Input
              value={channelLabel}
              onChange={(event) => onChannelLabelChange(event.target.value)}
              placeholder={t('channels:editor.placeholders.channelLabel')}
              disabled={channelActionDisabled}
            />
          </Label>
          <Label>
            <span>{t('channels:editor.audience')}</span>
            <Select
              aria-label={t('channels:editor.audience')}
              value={channelAudience}
              onChange={(event) => onChannelAudienceChange(event.target.value as ChannelAudienceOption['value'])}
              disabled={channelActionDisabled}
            >
              {channelAudienceOptions.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </Select>
          </Label>
          <Button variant='secondary' type='submit' disabled={channelActionDisabled}>
            {t('channels:actions.createChannel')}
          </Button>
        </form>

        <form className='composer composer-compact' onSubmit={onJoinInvite}>
          <Label>
            <span>{t('channels:editor.joinViaInvite')}</span>
            <Textarea
              value={inviteTokenInput}
              onChange={(event) => onInviteTokenChange(event.target.value)}
              placeholder={t('channels:editor.placeholders.inviteToken')}
              disabled={channelActionDisabled}
            />
          </Label>
          <div className='discovery-actions'>
            <Button
              variant='secondary'
              type='submit'
              disabled={channelActionDisabled}
            >
              {t('channels:actions.joinInvite')}
            </Button>
            <Button
              variant='secondary'
              type='button'
              disabled={channelActionDisabled}
              onClick={onJoinGrant}
            >
              {t('channels:actions.joinGrant')}
            </Button>
            <Button
              variant='secondary'
              type='button'
              disabled={channelActionDisabled}
              onClick={onJoinShare}
            >
              {t('channels:actions.joinShare')}
            </Button>
          </div>
        </form>

        {inviteOutput ? (
          <Notice tone='accent'>
            <strong>{audienceSummaryLabel(inviteOutputLabel, t)}</strong>
            <code className='extended-inline-code'>{inviteOutput}</code>
          </Notice>
        ) : null}

        {channels.length === 0 && status === 'ready' ? (
          <p className='empty-state'>{t('channels:empty')}</p>
        ) : null}

        {channels.length > 0 ? (
          <div className='extended-channel-grid'>
            <ul className='post-list'>
              {channels.map(({ channel, active }) => (
                <li key={channel.channel_id}>
                  <button
                    className={cn(
                      'post-card post-link extended-channel-card',
                      active && 'extended-channel-card-active'
                    )}
                    type='button'
                    aria-pressed={active}
                    onClick={() => onSelectChannel(channel.channel_id)}
                  >
                    <div className='post-meta'>
                      <span>{channel.label}</span>
                      <span>{t(`channels:audienceOptions.${channel.audience_kind}`)}</span>
                    </div>
                    <div className='topic-diagnostic topic-diagnostic-secondary'>
                      <span>{t('common:labels.epoch')}: {channel.current_epoch_id}</span>
                      <span>{t('common:labels.sharing')}: {channel.sharing_state}</span>
                    </div>
                  </button>
                </li>
              ))}
            </ul>

            <Card tone={selectedChannel ? 'accent' : 'default'} className='extended-channel-detail'>
            <CardHeader>
                <h4>{selectedChannel?.label ?? t('channels:selectChannel')}</h4>
                <small>
                  {selectedChannel
                    ? policyDescription(selectedChannel.audience_kind, t)
                    : t('channels:inspectHint')}
                </small>
              </CardHeader>

              {selectedChannel ? (
                <>
                  <div className='topic-diagnostic topic-diagnostic-secondary'>
                    <span>
                      {t('common:labels.policy')}: {policyDescription(selectedChannel.audience_kind, t)}
                    </span>
                    <span>{t('common:labels.epoch')}: {selectedChannel.current_epoch_id}</span>
                    <span>{t('common:labels.sharing')}: {selectedChannel.sharing_state}</span>
                    {selectedChannel.joined_via_pubkey ? (
                      <span>{t('common:labels.joinedVia')} {shortPubkey(selectedChannel.joined_via_pubkey)}</span>
                    ) : null}
                  </div>
                  {(selectedChannel.audience_kind === 'friend_only' ||
                    selectedChannel.audience_kind === 'friend_plus') ? (
                    <div className='topic-diagnostic topic-diagnostic-secondary'>
                      <span>{t('common:labels.participants')}: {selectedChannel.participant_count}</span>
                      <span>{t('common:labels.stale')}: {selectedChannel.stale_participant_count}</span>
                      <span>{t('common:labels.owner')}: {selectedChannel.is_owner ? t('common:states.yes') : t('common:states.no')}</span>
                    </div>
                  ) : null}
                  {selectedChannel.audience_kind === 'friend_only' &&
                  selectedChannel.rotation_required ? (
                    <div className='topic-diagnostic topic-diagnostic-error'>
                      <span>{t('channels:rotationRequired')}</span>
                    </div>
                  ) : null}
                  <div className='discovery-actions'>
                    {selectedChannel.audience_kind === 'invite_only' ? (
                      <Button
                        variant='secondary'
                        type='button'
                        disabled={channelActionDisabled || selectedChannelId === null}
                        onClick={onCreateInvite}
                      >
                        {t('channels:actions.createInvite')}
                      </Button>
                    ) : null}
                    {selectedChannel.audience_kind === 'friend_only' ? (
                      <Button
                        variant='secondary'
                        type='button'
                        disabled={channelActionDisabled || !selectedChannel.is_owner}
                        onClick={onCreateGrant}
                      >
                        {t('channels:actions.createGrant')}
                      </Button>
                    ) : null}
                    {selectedChannel.audience_kind === 'friend_plus' ? (
                      <Button
                        variant='secondary'
                        type='button'
                        disabled={channelActionDisabled || selectedChannelId === null}
                        onClick={onCreateShare}
                      >
                        {t('channels:actions.createShare')}
                      </Button>
                    ) : null}
                    {selectedChannel.audience_kind === 'friend_plus' ? (
                      <Button
                        variant='secondary'
                        type='button'
                        disabled={channelActionDisabled || !selectedChannel.is_owner}
                        onClick={onFreeze}
                      >
                        {t('common:actions.freeze')}
                      </Button>
                    ) : null}
                    {selectedChannel.audience_kind === 'friend_only' ||
                    selectedChannel.audience_kind === 'friend_plus' ? (
                      <Button
                        variant='secondary'
                        type='button'
                        disabled={channelActionDisabled || !selectedChannel.is_owner}
                        onClick={onRotate}
                      >
                        {t('common:actions.rotate')}
                      </Button>
                    ) : null}
                  </div>
                </>
              ) : (
                <Notice>{t('channels:selectChannelNotice')}</Notice>
              )}
            </Card>
          </div>
        ) : null}

        {status !== 'error' && error ? <p className='error error-inline'>{error}</p> : null}
      </div>
    </Card>
  );
}
