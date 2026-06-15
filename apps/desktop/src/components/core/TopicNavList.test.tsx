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
    'Disconnect kukuri:topic:demo from the gossip network'
  );
  expect(button).toHaveAttribute('aria-pressed', 'true');
  expect(button).toHaveClass('topic-plug-active');

  await user.click(button);
  expect(onToggleTopicGossip).toHaveBeenCalledWith('kukuri:topic:demo', false);
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

  const button = screen.getByLabelText('Connect kukuri:topic:demo to the gossip network');
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
