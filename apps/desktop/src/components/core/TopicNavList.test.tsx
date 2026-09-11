import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, test, vi } from 'vitest';

import { TopicNavList } from './TopicNavList';
import { type TopicDiagnosticSummary } from './types';

function topicItem(overrides?: Partial<TopicDiagnosticSummary>): TopicDiagnosticSummary {
  return {
    topic: 'kukuri:topic:demo',
    active: true,
    removable: true,
    connectionLabel: 'joined',
    peerCount: 2,
    lastReceivedLabel: '12:45:11',
    gossipJoined: true,
    channels: [
      {
        channelId: 'channel-1',
        label: 'Core',
        audienceKind: 'friend_plus',
        active: false,
        gossipJoined: true,
      },
    ],
    ...overrides,
  };
}

test('renders the topic plug as connected and disconnects on click', async () => {
  const user = userEvent.setup();
  const onToggleTopicGossip = vi.fn();

  render(
    <TopicNavList
      items={[topicItem()]}
      onSelectTopic={vi.fn()}
      onSelectChannel={vi.fn()}
      onRemoveTopic={vi.fn()}
      onToggleTopicGossip={onToggleTopicGossip}
    />
  );

  const button = screen.getByLabelText(
    'Disconnect demo from the gossip network'
  );
  expect(button).toHaveAttribute('aria-pressed', 'true');
  expect(button).toHaveClass('topic-plug-active');

  await user.click(button);
  expect(onToggleTopicGossip).toHaveBeenCalledWith('kukuri:topic:demo', false);
});

test('opens a public topic indexing request from topic management', async () => {
  const user = userEvent.setup();
  const onRequestTopicIndexing = vi.fn();
  render(
    <TopicNavList
      items={[topicItem()]}
      onSelectTopic={vi.fn()}
      onSelectChannel={vi.fn()}
      onRemoveTopic={vi.fn()}
      onRequestTopicIndexing={onRequestTopicIndexing}
    />
  );

  await user.click(screen.getByLabelText('Request indexing for demo'));
  expect(onRequestTopicIndexing).toHaveBeenCalledWith('kukuri:topic:demo');
});

test('renders the topic plug as disconnected and reconnects on click', async () => {
  const user = userEvent.setup();
  const onToggleTopicGossip = vi.fn();

  render(
    <TopicNavList
      items={[topicItem({ gossipJoined: false })]}
      onSelectTopic={vi.fn()}
      onSelectChannel={vi.fn()}
      onRemoveTopic={vi.fn()}
      onToggleTopicGossip={onToggleTopicGossip}
    />
  );

  const button = screen.getByLabelText('Connect demo to the gossip network');
  expect(button).toHaveAttribute('aria-pressed', 'false');
  expect(button).not.toHaveClass('topic-plug-active');

  await user.click(button);
  expect(onToggleTopicGossip).toHaveBeenCalledWith('kukuri:topic:demo', true);
});

test('toggles a channel gossip subscription', async () => {
  const user = userEvent.setup();
  const onToggleChannelGossip = vi.fn();

  render(
    <TopicNavList
      items={[
        topicItem({
          channels: [
            {
              channelId: 'channel-1',
              label: 'Core',
              audienceKind: 'friend_plus',
              active: false,
              gossipJoined: false,
            },
          ],
        }),
      ]}
      onSelectTopic={vi.fn()}
      onSelectChannel={vi.fn()}
      onRemoveTopic={vi.fn()}
      onToggleChannelGossip={onToggleChannelGossip}
    />
  );

  const button = screen.getByLabelText('Connect Core to the gossip network');
  expect(button).toHaveAttribute('aria-pressed', 'false');

  await user.click(button);
  expect(onToggleChannelGossip).toHaveBeenCalledWith('kukuri:topic:demo', 'channel-1', true);
});

// Issue #966: 参加済みチャンネルが無い topic にも、機能の存在と作成・参加の入口を出す。
test('shows the empty channel state with a create/join entry when no channel is joined', async () => {
  const user = userEvent.setup();
  const onOpenChannelManager = vi.fn();
  render(
    <TopicNavList
      items={[topicItem({ channels: [] })]}
      onSelectTopic={vi.fn()}
      onSelectChannel={vi.fn()}
      onRemoveTopic={vi.fn()}
      onOpenChannelManager={onOpenChannelManager}
    />
  );

  expect(screen.getByText('Channels')).toBeInTheDocument();
  expect(screen.getByText('No joined private channels.')).toBeInTheDocument();
  await user.click(screen.getByRole('button', { name: 'Create or join' }));
  expect(onOpenChannelManager).toHaveBeenCalledWith('kukuri:topic:demo');
});

test('omits the empty channel state when no channel manager entry is wired', () => {
  render(
    <TopicNavList
      items={[topicItem({ channels: [] })]}
      onSelectTopic={vi.fn()}
      onSelectChannel={vi.fn()}
      onRemoveTopic={vi.fn()}
    />
  );

  expect(screen.queryByText('No joined private channels.')).not.toBeInTheDocument();
  expect(screen.queryByRole('button', { name: 'Create or join' })).not.toBeInTheDocument();
});
