import type { FormEventHandler } from 'react';

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

function audienceSummaryLabel(label: InviteOutputLabel): string {
  if (label === 'grant') {
    return 'Latest grant';
  }
  if (label === 'share') {
    return 'Latest share';
  }
  return 'Latest invite';
}

function shortPubkey(pubkey: string): string {
  return pubkey.slice(0, 12);
}

function policyDescription(audienceKind: PrivateChannelListItemView['channel']['audience_kind']) {
  if (audienceKind === 'friend_only') {
    return 'Friends: only mutual followers can join';
  }
  if (audienceKind === 'friend_plus') {
    return 'Friends+: participants can share to their mutuals';
  }
  return 'Invite only';
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
  const channelActionDisabled = pendingAction !== null;
  const selectedChannelId = selectedChannel?.channel_id ?? null;

  return (
    <Card className='panel-subsection'>
      <CardHeader>
        <h3>Private Channels</h3>
        <small>{channels.length} joined</small>
      </CardHeader>

      {status === 'loading' ? <Notice>Loading private channels…</Notice> : null}
      {status === 'error' && error ? <Notice tone='destructive'>{error}</Notice> : null}

      <div className='extended-module-stack'>
        <form className='composer composer-compact' onSubmit={onCreateChannel}>
          <Label>
            <span>Create Channel</span>
            <Input
              value={channelLabel}
              onChange={(event) => onChannelLabelChange(event.target.value)}
              placeholder='core contributors'
              disabled={channelActionDisabled}
            />
          </Label>
          <Label>
            <span>Audience</span>
            <Select
              aria-label='Channel Audience'
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
            Create Channel
          </Button>
        </form>

        <form className='composer composer-compact' onSubmit={onJoinInvite}>
          <Label>
            <span>Join via Invite</span>
            <Textarea
              value={inviteTokenInput}
              onChange={(event) => onInviteTokenChange(event.target.value)}
              placeholder='paste private channel invite, friend grant, or friends+ share'
              disabled={channelActionDisabled}
            />
          </Label>
          <div className='discovery-actions'>
            <Button
              variant='secondary'
              type='submit'
              disabled={channelActionDisabled}
            >
              Join Invite
            </Button>
            <Button
              variant='secondary'
              type='button'
              disabled={channelActionDisabled}
              onClick={onJoinGrant}
            >
              Join Grant
            </Button>
            <Button
              variant='secondary'
              type='button'
              disabled={channelActionDisabled}
              onClick={onJoinShare}
            >
              Join Share
            </Button>
          </div>
        </form>

        {inviteOutput ? (
          <Notice tone='accent'>
            <strong>{audienceSummaryLabel(inviteOutputLabel)}</strong>
            <code className='extended-inline-code'>{inviteOutput}</code>
          </Notice>
        ) : null}

        {channels.length === 0 && status === 'ready' ? (
          <p className='empty-state'>No joined private channels for this topic.</p>
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
                      <span>{channel.audience_kind.replace('_', ' ')}</span>
                    </div>
                    <div className='topic-diagnostic topic-diagnostic-secondary'>
                      <span>epoch: {channel.current_epoch_id}</span>
                      <span>sharing: {channel.sharing_state}</span>
                    </div>
                  </button>
                </li>
              ))}
            </ul>

            <Card tone={selectedChannel ? 'accent' : 'default'} className='extended-channel-detail'>
              <CardHeader>
                <h4>{selectedChannel?.label ?? 'Select a channel'}</h4>
                <small>{selectedChannel ? policyDescription(selectedChannel.audience_kind) : 'Inspect policy and actions here.'}</small>
              </CardHeader>

              {selectedChannel ? (
                <>
                  <div className='topic-diagnostic topic-diagnostic-secondary'>
                    <span>Policy: {policyDescription(selectedChannel.audience_kind)}</span>
                    <span>epoch: {selectedChannel.current_epoch_id}</span>
                    <span>sharing: {selectedChannel.sharing_state}</span>
                    {selectedChannel.joined_via_pubkey ? (
                      <span>joined via {shortPubkey(selectedChannel.joined_via_pubkey)}</span>
                    ) : null}
                  </div>
                  {(selectedChannel.audience_kind === 'friend_only' ||
                    selectedChannel.audience_kind === 'friend_plus') ? (
                    <div className='topic-diagnostic topic-diagnostic-secondary'>
                      <span>participants: {selectedChannel.participant_count}</span>
                      <span>stale: {selectedChannel.stale_participant_count}</span>
                      <span>owner: {selectedChannel.is_owner ? 'yes' : 'no'}</span>
                    </div>
                  ) : null}
                  {selectedChannel.audience_kind === 'friend_only' &&
                  selectedChannel.rotation_required ? (
                    <div className='topic-diagnostic topic-diagnostic-error'>
                      <span>rotation required: current participants include non-mutual followers</span>
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
                        Create Invite
                      </Button>
                    ) : null}
                    {selectedChannel.audience_kind === 'friend_only' ? (
                      <Button
                        variant='secondary'
                        type='button'
                        disabled={channelActionDisabled || !selectedChannel.is_owner}
                        onClick={onCreateGrant}
                      >
                        Create Grant
                      </Button>
                    ) : null}
                    {selectedChannel.audience_kind === 'friend_plus' ? (
                      <Button
                        variant='secondary'
                        type='button'
                        disabled={channelActionDisabled || selectedChannelId === null}
                        onClick={onCreateShare}
                      >
                        Create Share
                      </Button>
                    ) : null}
                    {selectedChannel.audience_kind === 'friend_plus' ? (
                      <Button
                        variant='secondary'
                        type='button'
                        disabled={channelActionDisabled || !selectedChannel.is_owner}
                        onClick={onFreeze}
                      >
                        Freeze
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
                        Rotate
                      </Button>
                    ) : null}
                  </div>
                </>
              ) : (
                <Notice>Select a private channel to inspect policy and actions.</Notice>
              )}
            </Card>
          </div>
        ) : null}

        {status !== 'error' && error ? <p className='error error-inline'>{error}</p> : null}
      </div>
    </Card>
  );
}
