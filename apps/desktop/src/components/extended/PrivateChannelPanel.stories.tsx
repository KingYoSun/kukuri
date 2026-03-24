import { useState, type FormEvent } from 'react';

import type { Meta, StoryObj } from '@storybook/react-vite';

import { PrivateChannelPanel } from './PrivateChannelPanel';
import type { InviteOutputLabel, PrivateChannelListItemView } from './types';

const meta = {
  title: 'Extended/PrivateChannelPanel',
  component: PrivateChannelPanel,
} satisfies Meta<typeof PrivateChannelPanel>;

export default meta;

type Story = StoryObj<typeof meta>;

const BASE_CHANNELS: PrivateChannelListItemView[] = [
  {
    active: true,
    channel: {
      topic_id: 'kukuri:topic:demo',
      channel_id: 'channel-1',
      label: 'Core Contributors',
      creator_pubkey: 'a'.repeat(64),
      owner_pubkey: 'a'.repeat(64),
      joined_via_pubkey: null,
      audience_kind: 'friend_plus',
      is_owner: true,
      current_epoch_id: 'epoch-4',
      archived_epoch_ids: ['epoch-3'],
      sharing_state: 'open',
      rotation_required: false,
      participant_count: 3,
      stale_participant_count: 0,
    },
  },
  {
    active: false,
    channel: {
      topic_id: 'kukuri:topic:demo',
      channel_id: 'channel-2',
      label: 'Invite-only Review',
      creator_pubkey: 'b'.repeat(64),
      owner_pubkey: 'b'.repeat(64),
      joined_via_pubkey: 'c'.repeat(64),
      audience_kind: 'invite_only',
      is_owner: false,
      current_epoch_id: 'legacy',
      archived_epoch_ids: [],
      sharing_state: 'open',
      rotation_required: false,
      participant_count: 0,
      stale_participant_count: 0,
    },
  },
];

const STORY_ARGS = {
  status: 'ready',
  error: null,
  pendingAction: null,
  channelLabel: 'Core Contributors',
  channelAudience: 'friend_plus',
  channelAudienceOptions: [
    { value: 'invite_only', label: 'Invite only' },
    { value: 'friend_only', label: 'Friends' },
    { value: 'friend_plus', label: 'Friends+' },
  ],
  inviteTokenInput: '',
  inviteOutput: null,
  inviteOutputLabel: 'invite',
  channels: BASE_CHANNELS,
  selectedChannel: BASE_CHANNELS[0].channel,
  onChannelLabelChange: () => undefined,
  onChannelAudienceChange: () => undefined,
  onInviteTokenChange: () => undefined,
  onCreateChannel: (event: FormEvent<HTMLFormElement>) => event.preventDefault(),
  onJoinInvite: (event: FormEvent<HTMLFormElement>) => event.preventDefault(),
  onJoinGrant: () => undefined,
  onJoinShare: () => undefined,
  onSelectChannel: () => undefined,
  onCreateInvite: () => undefined,
  onCreateGrant: () => undefined,
  onCreateShare: () => undefined,
  onFreeze: () => undefined,
  onRotate: () => undefined,
} satisfies React.ComponentProps<typeof PrivateChannelPanel>;

function ChannelStory({
  status = 'ready',
  error = null,
  inviteOutput = null,
  inviteOutputLabel = 'invite',
}: {
  status?: 'loading' | 'ready' | 'error';
  error?: string | null;
  inviteOutput?: string | null;
  inviteOutputLabel?: InviteOutputLabel;
}) {
  const [label, setLabel] = useState('Core Contributors');
  const [audience, setAudience] = useState<'invite_only' | 'friend_only' | 'friend_plus'>(
    'friend_plus'
  );
  const [token, setToken] = useState('');

  return (
    <PrivateChannelPanel
      status={status}
      error={error}
      pendingAction={null}
      channelLabel={label}
      channelAudience={audience}
      channelAudienceOptions={[
        { value: 'invite_only', label: 'Invite only' },
        { value: 'friend_only', label: 'Friends' },
        { value: 'friend_plus', label: 'Friends+' },
      ]}
      inviteTokenInput={token}
      inviteOutput={inviteOutput}
      inviteOutputLabel={inviteOutputLabel}
      channels={BASE_CHANNELS}
      selectedChannel={BASE_CHANNELS[0].channel}
      onChannelLabelChange={setLabel}
      onChannelAudienceChange={setAudience}
      onInviteTokenChange={setToken}
      onCreateChannel={(event) => event.preventDefault()}
      onJoinInvite={(event) => event.preventDefault()}
      onJoinGrant={() => undefined}
      onJoinShare={() => undefined}
      onSelectChannel={() => undefined}
      onCreateInvite={() => undefined}
      onCreateGrant={() => undefined}
      onCreateShare={() => undefined}
      onFreeze={() => undefined}
      onRotate={() => undefined}
    />
  );
}

export const Ready: Story = {
  args: STORY_ARGS,
  render: (args) => (
    <ChannelStory
      status={args.status}
      error={args.error}
      inviteOutput={args.inviteOutput}
      inviteOutputLabel={args.inviteOutputLabel}
    />
  ),
};

export const ErrorState: Story = {
  args: {
    ...STORY_ARGS,
    status: 'error',
    error: 'private channel refresh failed',
  },
  render: (args) => (
    <ChannelStory
      status={args.status}
      error={args.error}
      inviteOutput={args.inviteOutput}
      inviteOutputLabel={args.inviteOutputLabel}
    />
  ),
};

export const InviteOutputState: Story = {
  args: {
    ...STORY_ARGS,
    inviteOutput: 'share:kukuri:topic:demo:channel-1',
    inviteOutputLabel: 'share',
  },
  render: (args) => (
    <ChannelStory
      status={args.status}
      error={args.error}
      inviteOutput={args.inviteOutput}
      inviteOutputLabel={args.inviteOutputLabel}
    />
  ),
};
