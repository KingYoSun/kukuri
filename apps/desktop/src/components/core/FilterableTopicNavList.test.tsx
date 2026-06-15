import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, test, vi } from 'vitest';

import { FilterableTopicNavList } from './FilterableTopicNavList';
import { type TopicDiagnosticSummary } from './types';

function topicItem(overrides: Partial<TopicDiagnosticSummary> & { topic: string }): TopicDiagnosticSummary {
  return {
    active: false,
    removable: true,
    connectionLabel: 'joined',
    peerCount: 1,
    lastReceivedLabel: '12:00:00',
    lastReceivedAt: null,
    gossipJoined: true,
    channels: [],
    ...overrides,
  };
}

const items: TopicDiagnosticSummary[] = [
  topicItem({
    topic: 'kukuri:topic:gamma',
    lastReceivedAt: 200,
    channels: [{ channelId: 'c1', label: 'SecretRoom', audienceKind: 'friend_plus', active: false }],
  }),
  topicItem({ topic: 'kukuri:topic:alpha', lastReceivedAt: 100 }),
  topicItem({ topic: 'kukuri:topic:beta', lastReceivedAt: 300, gossipJoined: false }),
];

function renderList() {
  return render(
    <FilterableTopicNavList
      items={items}
      onSelectTopic={vi.fn()}
      onSelectChannel={vi.fn()}
      onRemoveTopic={vi.fn()}
    />
  );
}

// Topic labels carry a `title` attribute; channel/public-scope labels do not,
// so filtering on [title] isolates the top-level topic order.
function topicOrder(container: HTMLElement): string[] {
  return Array.from(container.querySelectorAll('.shell-topic-link-label[title]')).map(
    (el) => el.getAttribute('title') ?? ''
  );
}

test('renders topics in added order by default', () => {
  const { container } = renderList();
  expect(topicOrder(container)).toEqual([
    'kukuri:topic:gamma',
    'kukuri:topic:alpha',
    'kukuri:topic:beta',
  ]);
});

test('searches by topic name', async () => {
  const user = userEvent.setup();
  const { container } = renderList();

  await user.type(screen.getByLabelText('Search topics'), 'beta');
  expect(topicOrder(container)).toEqual(['kukuri:topic:beta']);
});

test('searches by channel name', async () => {
  const user = userEvent.setup();
  const { container } = renderList();

  await user.type(screen.getByLabelText('Search topics'), 'secretroom');
  expect(topicOrder(container)).toEqual(['kukuri:topic:gamma']);
});

test('filters by connection state', async () => {
  const user = userEvent.setup();
  const { container } = renderList();

  await user.selectOptions(screen.getByLabelText('Filter by connection'), 'connected');
  expect(topicOrder(container)).toEqual(['kukuri:topic:gamma', 'kukuri:topic:alpha']);

  await user.selectOptions(screen.getByLabelText('Filter by connection'), 'disconnected');
  expect(topicOrder(container)).toEqual(['kukuri:topic:beta']);
});

test('sorts by name and by last updated', async () => {
  const user = userEvent.setup();
  const { container } = renderList();

  await user.selectOptions(screen.getByLabelText('Sort'), 'name');
  expect(topicOrder(container)).toEqual([
    'kukuri:topic:alpha',
    'kukuri:topic:beta',
    'kukuri:topic:gamma',
  ]);

  await user.selectOptions(screen.getByLabelText('Sort'), 'updated');
  expect(topicOrder(container)).toEqual([
    'kukuri:topic:beta',
    'kukuri:topic:gamma',
    'kukuri:topic:alpha',
  ]);
});

test('shows an empty message when nothing matches', async () => {
  const user = userEvent.setup();
  renderList();

  await user.type(screen.getByLabelText('Search topics'), 'no-such-topic');
  expect(screen.getByText('No topics match your filters.')).toBeInTheDocument();
});

test('hides controls when only one topic is tracked', () => {
  render(
    <FilterableTopicNavList
      items={[topicItem({ topic: 'kukuri:topic:solo' })]}
      onSelectTopic={vi.fn()}
      onSelectChannel={vi.fn()}
      onRemoveTopic={vi.fn()}
    />
  );

  expect(screen.queryByLabelText('Search topics')).not.toBeInTheDocument();
});
