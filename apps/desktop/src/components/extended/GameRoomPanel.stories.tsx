import { useState, type FormEvent } from 'react';

import type { Meta, StoryObj } from '@storybook/react-vite';

import { GameRoomPanel } from './GameRoomPanel';

const meta = {
  title: 'Extended/GameRoomPanel',
  component: GameRoomPanel,
} satisfies Meta<typeof GameRoomPanel>;

export default meta;

type Story = StoryObj<typeof meta>;

const STORY_TIMESTAMP = 1_742_860_800_000;

const STORY_ARGS = {
  status: 'ready',
  error: null,
  audienceLabel: 'Core Contributors',
  title: 'Top 8 Finals',
  description: 'set one',
  participantsInput: 'Alice, Bob',
  createPending: false,
  rooms: [
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
  ],
  drafts: {
    'game-1': {
      status: 'Running',
      phaseLabel: 'Round 3',
      scores: {
        alice: '2',
        bob: '1',
      },
    },
  },
  savingByRoomId: {},
  localAuthorPubkey: 'f'.repeat(64),
  onTitleChange: () => undefined,
  onDescriptionChange: () => undefined,
  onParticipantsChange: () => undefined,
  onSubmit: (event: FormEvent<HTMLFormElement>) => event.preventDefault(),
  onDraftStatusChange: () => undefined,
  onDraftPhaseChange: () => undefined,
  onDraftScoreChange: () => undefined,
  onSaveRoom: () => undefined,
} satisfies React.ComponentProps<typeof GameRoomPanel>;

function GameStory({
  status = 'ready',
  error = null,
}: {
  status?: 'loading' | 'ready' | 'error';
  error?: string | null;
}) {
  const [title, setTitle] = useState('Top 8 Finals');
  const [description, setDescription] = useState('set one');
  const [participants, setParticipants] = useState('Alice, Bob');

  return (
    <GameRoomPanel
      status={status}
      error={error}
      audienceLabel='Core Contributors'
      title={title}
      description={description}
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
      onTitleChange={setTitle}
      onDescriptionChange={setDescription}
      onParticipantsChange={setParticipants}
      onSubmit={(event) => event.preventDefault()}
      onDraftStatusChange={() => undefined}
      onDraftPhaseChange={() => undefined}
      onDraftScoreChange={() => undefined}
      onSaveRoom={() => undefined}
    />
  );
}

export const Ready: Story = {
  args: STORY_ARGS,
  render: (args) => <GameStory status={args.status} error={args.error} />,
};

export const ErrorState: Story = {
  args: {
    ...STORY_ARGS,
    status: 'error',
    error: 'game room sync failed',
  },
  render: (args) => <GameStory status={args.status} error={args.error} />,
};
