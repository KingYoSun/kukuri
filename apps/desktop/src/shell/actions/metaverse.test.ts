import { describe, expect, test, vi } from 'vitest';

import { createDesktopMockApi } from '@/mocks/desktopApiMock';
import { createMetaverseRoomActions } from './metaverse';
import { createDefaultDomeCustomization } from '@/components/extended/metaverse/DomeSceneModel';

describe('createMetaverseRoomActions', () => {
  test('binds topic/channel and preserves every API payload and refresh boundary', async () => {
    const baseApi = createDesktopMockApi();
    const api = {
      ...baseApi,
      createMetaverseRoom: vi.fn(baseApi.createMetaverseRoom),
      publishMetaverseRoomEvent: vi.fn(baseApi.publishMetaverseRoomEvent),
      listMetaverseRoomEvents: vi.fn(baseApi.listMetaverseRoomEvents),
      importMetaverseRoomAsset: vi.fn(baseApi.importMetaverseRoomAsset),
      getBlobPreviewUrl: vi.fn(baseApi.getBlobPreviewUrl),
      updateMetaverseRoom: vi.fn(baseApi.updateMetaverseRoom),
      moveDome: vi.fn().mockResolvedValue(null as never),
    };
    const onRefresh = vi.fn().mockResolvedValue(undefined);
    const actions = createMetaverseRoomActions({
      api,
      activeTopic: 'kukuri:topic:general',
      activeComposeChannel: { kind: 'private_channel', channel_id: 'channel-1' },
      onRefresh,
    });
    const event = {
      type: 'presence_leave' as const,
      room_id: 'room-1',
      peer_id: 'peer-1',
      left_at: 7,
    };

    await actions.createRoom({
      title: 'Atrium',
      description: 'Small social space',
      maxPeers: 8,
    });
    await actions.publishRoomEvent('room-1', 'peer-1', 7, event);
    await actions.listRoomEvents('room-1', 'event-6', 64);
    await actions.importRoomAsset('room-1', 'vrm', 'model/vrm', 'avatar.vrm', 'YXZhdGFy');
    await actions.getBlobPreviewUrl('blob-hash', 'model/vrm');
    const customization = createDefaultDomeCustomization();
    await actions.updateRoom('room-1', 'Waiting', customization);
    await actions.moveRoom('move-1', 'room-1', {
      kind: 'channel',
      topic_id: 'kukuri:topic:target',
      channel_id: 'channel-2',
    });

    expect(api.createMetaverseRoom).toHaveBeenCalledWith(
      'kukuri:topic:general',
      'Atrium',
      'Small social space',
      8,
      { kind: 'private_channel', channel_id: 'channel-1' }
    );
    expect(api.publishMetaverseRoomEvent).toHaveBeenCalledWith(
      'kukuri:topic:general',
      'room-1',
      'peer-1',
      7,
      event
    );
    expect(api.listMetaverseRoomEvents).toHaveBeenCalledWith(
      'kukuri:topic:general',
      'room-1',
      'event-6',
      64
    );
    expect(api.importMetaverseRoomAsset).toHaveBeenCalledWith(
      'kukuri:topic:general',
      'room-1',
      'vrm',
      'model/vrm',
      'avatar.vrm',
      'YXZhdGFy'
    );
    expect(api.getBlobPreviewUrl).toHaveBeenCalledWith('blob-hash', 'model/vrm', undefined);
    expect(api.updateMetaverseRoom).toHaveBeenCalledWith(
      'kukuri:topic:general',
      'room-1',
      'Waiting',
      customization
    );
    expect(api.moveDome).toHaveBeenCalledWith(
      'kukuri:topic:general',
      'move-1',
      'room-1',
      {
        kind: 'channel',
        topic_id: 'kukuri:topic:target',
        channel_id: 'channel-2',
      }
    );
    expect(onRefresh).not.toHaveBeenCalled();

    await actions.refresh();
    expect(onRefresh).toHaveBeenCalledTimes(1);
  });
});
