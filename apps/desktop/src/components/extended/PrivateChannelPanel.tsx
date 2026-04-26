import type { FormEventHandler } from 'react';
import { useTranslation } from 'react-i18next';
import { Copy } from 'lucide-react';

import { buildChannelAccessPreviewDeepLink } from '@/lib/internalLinks';
import { Button } from '@/components/ui/button';
import { Card, CardHeader } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Notice } from '@/components/ui/notice';
import { Select } from '@/components/ui/select';
import { Textarea } from '@/components/ui/textarea';

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
  if (label === 'grant' || label === 'share') {
    return t('latestShare');
  }
  return t('latestInvite');
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
  inviteOutput?: string | null;
  inviteOutputLabel?: InviteOutputLabel;
  onChannelLabelChange: (value: string) => void;
  onChannelAudienceChange: (value: ChannelAudienceOption['value']) => void;
  onInviteTokenChange: (value: string) => void;
  onCreateChannel: FormEventHandler<HTMLFormElement>;
  onJoin: FormEventHandler<HTMLFormElement>;
  onCopyInviteOutput?: (token: string) => void;
};

type PrivateChannelSettingsPanelProps = {
  error: string | null;
  pendingAction: PrivateChannelPendingAction;
  channel: PrivateChannelListItemView['channel'];
  inviteOutput: string | null;
  inviteOutputLabel: InviteOutputLabel;
  onShare: () => void;
  onCopyInviteOutput?: (token: string) => void;
};

export function PrivateChannelPanel({
  status,
  error,
  pendingAction,
  channelLabel,
  channelAudience,
  channelAudienceOptions,
  inviteTokenInput,
  inviteOutput = null,
  inviteOutputLabel = 'invite',
  onChannelLabelChange,
  onChannelAudienceChange,
  onInviteTokenChange,
  onCreateChannel,
  onJoin,
  onCopyInviteOutput,
}: PrivateChannelPanelProps) {
  const { t } = useTranslation(['channels', 'common']);
  const channelActionDisabled = pendingAction !== null;
  const channelAccessDeepLink = inviteOutput
    ? buildChannelAccessPreviewDeepLink(inviteOutput)
    : null;

  return (
    <div className='extended-module-stack'>
      {status === 'loading' ? <Notice>{t('channels:loading')}</Notice> : null}
      {status === 'error' && error ? <Notice tone='destructive'>{error}</Notice> : null}

      <div className='private-channel-editor-grid'>
        <Card className='panel-subsection private-channel-editor-block'>
          <CardHeader>
            <h3>{t('channels:editor.createBlockTitle')}</h3>
          </CardHeader>
          <form className='composer composer-compact' onSubmit={onCreateChannel}>
            <Label>
              <span>{t('channels:editor.channelName')}</span>
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
                onChange={(event) =>
                  onChannelAudienceChange(event.target.value as ChannelAudienceOption['value'])
                }
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
        </Card>

        <Card className='panel-subsection private-channel-editor-block'>
          <CardHeader>
            <h3>{t('channels:editor.joinBlockTitle')}</h3>
          </CardHeader>
          <form className='composer composer-compact' onSubmit={onJoin}>
            <Label>
              <span>{t('channels:editor.join')}</span>
              <Textarea
                value={inviteTokenInput}
                onChange={(event) => onInviteTokenChange(event.target.value)}
                placeholder={t('channels:editor.placeholders.inviteToken')}
                disabled={channelActionDisabled}
              />
            </Label>
            <Button variant='secondary' type='submit' disabled={channelActionDisabled}>
              {t('channels:actions.join')}
            </Button>
          </form>
        </Card>
      </div>

      {inviteOutput && channelAccessDeepLink ? (
        <Notice tone='accent'>
          <div className='shell-inline-actions'>
            <strong>{t('channels:copyShareLink')}</strong>
            {onCopyInviteOutput ? (
              <Button
                variant='secondary'
                size='icon'
                className='post-action-button'
                type='button'
                aria-label={t('common:actions.copyLink')}
                onClick={() => onCopyInviteOutput(channelAccessDeepLink)}
              >
                <Copy className='size-4' aria-hidden='true' />
              </Button>
            ) : null}
          </div>
          <span className='sr-only'>{audienceSummaryLabel(inviteOutputLabel, t)}</span>
        </Notice>
      ) : null}

      {status !== 'error' && error ? <p className='error error-inline'>{error}</p> : null}
    </div>
  );
}

export function PrivateChannelSettingsPanel({
  error,
  pendingAction,
  channel,
  inviteOutput,
  inviteOutputLabel,
  onShare,
  onCopyInviteOutput,
}: PrivateChannelSettingsPanelProps) {
  const { t } = useTranslation(['channels', 'common']);
  const channelActionDisabled = pendingAction !== null;
  const policyLabel = policyDescription(channel.audience_kind, t);
  const channelAccessDeepLink = inviteOutput
    ? buildChannelAccessPreviewDeepLink(inviteOutput)
    : null;

  return (
    <Card tone='accent' className='panel-subsection extended-channel-detail'>
      <CardHeader>
        <h3>{t('channels:settings.channelName', { channel: channel.label })}</h3>
        <small>{t('channels:settings.policy', { policy: policyLabel })}</small>
      </CardHeader>

      <div className='extended-module-stack'>
        {(channel.audience_kind === 'friend_only' || channel.audience_kind === 'friend_plus') ? (
          <div className='topic-diagnostic topic-diagnostic-secondary'>
            <span>{t('common:labels.participants')}: {channel.participant_count}</span>
            <span>{t('common:labels.stale')}: {channel.stale_participant_count}</span>
            <span>
              {t('common:labels.owner')}: {channel.is_owner ? t('common:states.yes') : t('common:states.no')}
            </span>
          </div>
        ) : null}
        {channel.audience_kind === 'friend_only' && channel.rotation_required ? (
          <div className='topic-diagnostic topic-diagnostic-error'>
            <span>{t('channels:rotationRequired')}</span>
          </div>
        ) : null}

        <div className='discovery-actions'>
          <Button
            aria-label={t('channels:actions.createShareLink')}
            className='w-full'
            variant='secondary'
            type='button'
            disabled={channelActionDisabled}
            onClick={onShare}
          >
            {t('channels:actions.createShareLink')}
          </Button>
        </div>

        {inviteOutput && channelAccessDeepLink ? (
          <Notice tone='accent'>
            <div className='shell-inline-actions'>
              <strong>{t('channels:copyShareLink')}</strong>
              {onCopyInviteOutput ? (
                <Button
                  variant='secondary'
                  size='icon'
                  className='post-action-button'
                  type='button'
                  aria-label={t('common:actions.copyLink')}
                  onClick={() => onCopyInviteOutput(channelAccessDeepLink)}
                >
                  <Copy className='size-4' aria-hidden='true' />
                </Button>
              ) : null}
            </div>
            <span className='sr-only'>{audienceSummaryLabel(inviteOutputLabel, t)}</span>
          </Notice>
        ) : null}

        {error ? <p className='error error-inline'>{error}</p> : null}
      </div>
    </Card>
  );
}
