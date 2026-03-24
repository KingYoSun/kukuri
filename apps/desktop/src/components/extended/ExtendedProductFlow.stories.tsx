import { useState } from 'react';

import type { Meta, StoryObj } from '@storybook/react-vite';

import { GameRoomPanel } from './GameRoomPanel';
import { LiveSessionPanel } from './LiveSessionPanel';
import { PrivateChannelPanel } from './PrivateChannelPanel';
import { ProfileEditorPanel } from './ProfileEditorPanel';

const meta = {
  title: 'Extended/ExtendedProductFlow',
  parameters: {
    layout: 'fullscreen',
  },
} satisfies Meta;

export default meta;

type Story = StoryObj<typeof meta>;

const STORY_TIMESTAMP = 1_742_860_800_000;

function ExtendedProductFlowStory({ width }: { width: number }) {
  const [profileFields, setProfileFields] = useState({
    displayName: 'Local Author',
    name: 'local-author',
    about: 'Maintains the desktop shell migration.',
    picture: 'https://example.com/avatar.png',
  });
  const [channelLabel, setChannelLabel] = useState('Core Contributors');
  const [inviteToken, setInviteToken] = useState('');
  const [liveTitle, setLiveTitle] = useState('Launch Party');
  const [liveDescription, setLiveDescription] = useState('watch party');
  const [gameTitle, setGameTitle] = useState('Top 8 Finals');
  const [gameDescription, setGameDescription] = useState('set one');
  const [participants, setParticipants] = useState('Alice, Bob');

  return (
    <div style={{ maxWidth: `${width}px`, margin: '0 auto' }}>
      <div className='shell-main-stack'>
        <PrivateChannelPanel
          status='ready'
          error={null}
          pendingAction={null}
          channelLabel={channelLabel}
          channelAudience='friend_plus'
          channelAudienceOptions={[
            { value: 'invite_only', label: 'Invite only' },
            { value: 'friend_only', label: 'Friends' },
            { value: 'friend_plus', label: 'Friends+' },
          ]}
          inviteTokenInput={inviteToken}
          inviteOutput='share:kukuri:topic:demo:channel-1'
          inviteOutputLabel='share'
          channels={[
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
          ]}
          selectedChannel={{
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
          }}
          onChannelLabelChange={setChannelLabel}
          onChannelAudienceChange={() => undefined}
          onInviteTokenChange={setInviteToken}
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

        <LiveSessionPanel
          status='ready'
          error={null}
          audienceLabel='Core Contributors'
          title={liveTitle}
          description={liveDescription}
          createPending={false}
          sessions={[
            {
              session: {
                session_id: 'live-1',
                host_pubkey: 'f'.repeat(64),
                title: 'Launch Party',
                description: 'watch party',
                status: 'Live',
                started_at: STORY_TIMESTAMP,
                ended_at: null,
                viewer_count: 4,
                joined_by_me: true,
                channel_id: 'channel-1',
                audience_label: 'Core Contributors',
              },
              isOwner: true,
              pending: false,
            },
          ]}
          onTitleChange={setLiveTitle}
          onDescriptionChange={setLiveDescription}
          onSubmit={(event) => event.preventDefault()}
          onJoin={() => undefined}
          onLeave={() => undefined}
          onEnd={() => undefined}
        />

        <GameRoomPanel
          status='ready'
          error={null}
          audienceLabel='Core Contributors'
          title={gameTitle}
          description={gameDescription}
          participantsInput={participants}
          createPending={false}
          rooms={[
            {
              room_id: 'game-1',
              host_pubkey: 'f'.repeat(64),
              title: 'Grand Finals',
              description: 'set one',
              status: 'Running',
              phase_label: 'Round 3',
              scores: [
                { participant_id: 'alice', label: 'Alice', score: 2 },
                { participant_id: 'bob', label: 'Bob', score: 1 },
              ],
              updated_at: STORY_TIMESTAMP,
              channel_id: 'channel-1',
              audience_label: 'Core Contributors',
            },
          ]}
          drafts={{
            'game-1': {
              status: 'Running',
              phaseLabel: 'Round 3',
              scores: {
                alice: '2',
                bob: '1',
              },
            },
          }}
          savingByRoomId={{}}
          localAuthorPubkey={'f'.repeat(64)}
          onTitleChange={setGameTitle}
          onDescriptionChange={setGameDescription}
          onParticipantsChange={setParticipants}
          onSubmit={(event) => event.preventDefault()}
          onDraftStatusChange={() => undefined}
          onDraftPhaseChange={() => undefined}
          onDraftScoreChange={() => undefined}
          onSaveRoom={() => undefined}
        />

        <ProfileEditorPanel
          authorLabel='Local Author'
          status='ready'
          saving={false}
          dirty={true}
          error={null}
          fields={profileFields}
          onFieldChange={(field, value) =>
            setProfileFields((current) => ({ ...current, [field]: value }))
          }
          onSave={(event) => event.preventDefault()}
          onReset={() => undefined}
        />
      </div>
    </div>
  );
}

export const WideWorkspace: Story = {
  render: () => <ExtendedProductFlowStory width={1180} />,
};

export const NarrowWorkspace: Story = {
  render: () => <ExtendedProductFlowStory width={760} />,
};
