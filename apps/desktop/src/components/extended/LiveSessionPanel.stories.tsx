import { useState, type FormEvent } from 'react';

import type { Meta, StoryObj } from '@storybook/react-vite';

import { LiveSessionPanel } from './LiveSessionPanel';

const meta = {
  title: 'Extended/LiveSessionPanel',
  component: LiveSessionPanel,
} satisfies Meta<typeof LiveSessionPanel>;

export default meta;

type Story = StoryObj<typeof meta>;

const STORY_ARGS = {
  status: 'ready',
  error: null,
  audienceLabel: 'Core Contributors',
  title: 'Friday stream',
  description: 'watch party',
  createPending: false,
  sessions: [
    {
      session: {
        session_id: 'live-1',
        host_pubkey: 'f'.repeat(64),
        title: 'Launch Party',
        description: 'Watch along',
        status: 'Live',
        started_at: Date.now(),
        ended_at: null,
        viewer_count: 4,
        joined_by_me: true,
        channel_id: 'channel-1',
        audience_label: 'Core Contributors',
      },
      isOwner: true,
      pending: false,
    },
  ],
  onTitleChange: () => undefined,
  onDescriptionChange: () => undefined,
  onSubmit: (event: FormEvent<HTMLFormElement>) => event.preventDefault(),
  onJoin: () => undefined,
  onLeave: () => undefined,
  onEnd: () => undefined,
} satisfies React.ComponentProps<typeof LiveSessionPanel>;

function LiveStory({
  status = 'ready',
  error = null,
}: {
  status?: 'loading' | 'ready' | 'error';
  error?: string | null;
}) {
  const [title, setTitle] = useState('Friday stream');
  const [description, setDescription] = useState('watch party');

  return (
    <LiveSessionPanel
      status={status}
      error={error}
      audienceLabel='Core Contributors'
      title={title}
      description={description}
      createPending={false}
      sessions={[
        {
          session: {
            session_id: 'live-1',
            host_pubkey: 'f'.repeat(64),
            title: 'Launch Party',
            description: 'Watch along',
            status: 'Live',
            started_at: Date.now(),
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
      onTitleChange={setTitle}
      onDescriptionChange={setDescription}
      onSubmit={(event) => event.preventDefault()}
      onJoin={() => undefined}
      onLeave={() => undefined}
      onEnd={() => undefined}
    />
  );
}

export const Ready: Story = {
  args: STORY_ARGS,
  render: (args) => <LiveStory status={args.status} error={args.error} />,
};

export const ErrorState: Story = {
  args: {
    ...STORY_ARGS,
    status: 'error',
    error: 'live session refresh failed',
  },
  render: (args) => <LiveStory status={args.status} error={args.error} />,
};
