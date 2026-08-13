import { afterEach, expect, test, vi } from 'vitest';

import { invoke } from '@tauri-apps/api/core';

import { runtimeApi } from './runtimeApi';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(async () => undefined),
}));

const invokeMock = vi.mocked(invoke);

afterEach(() => {
  invokeMock.mockClear();
  delete window.__KUKURI_DESKTOP__;
});

test('setTopicGossipEnabled invokes the desktop command', async () => {
  await runtimeApi.setTopicGossipEnabled('kukuri:topic:demo', false);
  expect(invokeMock).toHaveBeenCalledWith('set_topic_gossip_enabled', {
    request: { topic: 'kukuri:topic:demo', enabled: false },
  });
});

test('setChannelGossipEnabled invokes the desktop command', async () => {
  await runtimeApi.setChannelGossipEnabled('kukuri:topic:demo', 'channel-1', true);
  expect(invokeMock).toHaveBeenCalledWith('set_channel_gossip_enabled', {
    request: { topic: 'kukuri:topic:demo', channel: 'channel-1', enabled: true },
  });
});

test('searchCommunityNodeIndex invokes the typed index command', async () => {
  await runtimeApi.searchCommunityNodeIndex({
    base_url: 'https://node.example',
    query: 'hello',
    scope_kind: 'public_topic',
    scope_id: 'rust',
    limit: 20,
  });
  expect(invokeMock).toHaveBeenCalledWith('search_community_node_index', {
    request: {
      base_url: 'https://node.example',
      query: 'hello',
      scope_kind: 'public_topic',
      scope_id: 'rust',
      limit: 20,
    },
  });
});

test('submitCommunityNodeIndexingRequest invokes the typed indexing request command', async () => {
  await runtimeApi.submitCommunityNodeIndexingRequest({
    base_url: 'https://node.example',
    scope_kind: 'private_channel',
    topic_id: 'kukuri:topic:demo',
    channel_id: 'channel-1',
    confirm_private_channel_secret_disclosure: true,
  });
  expect(invokeMock).toHaveBeenCalledWith('submit_community_node_indexing_request', {
    request: {
      base_url: 'https://node.example',
      scope_kind: 'private_channel',
      topic_id: 'kukuri:topic:demo',
      channel_id: 'channel-1',
      confirm_private_channel_secret_disclosure: true,
    },
  });
});
