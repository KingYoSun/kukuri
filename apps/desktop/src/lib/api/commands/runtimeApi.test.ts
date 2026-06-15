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
