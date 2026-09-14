import type { Meta, StoryObj } from '@storybook/react-vite';
import { useMemo } from 'react';
import { createDesktopMockApi } from '@/mocks/desktopApiMock';
import { createMetaverseRoomActions } from '@/shell/actions/metaverse';
import { createDefaultMetaverseRoomState } from './DomeSceneModel';
import { DomeManagementPanel } from './DomeManagementPanel';
import type { GameRoomView } from '@/lib/api';

function ManagementStory({ running = false, unavailable = false }) {
  const room = useMemo<GameRoomView>(() => ({ room_id: 'management-review', host_pubkey: 'a'.repeat(64),
    title: '自分のDome — 作業と会話の空間', description: '', status: 'Waiting', scores: [], room_kind: 'metaverse_room',
    manifest_blob_hash: 'review', updated_at: 1, channel_id: null, audience_label: 'Public',
    metaverse: createDefaultMetaverseRoomState(8), dome_hosting: { kind: running ? 'owner_hosted' : 'closed' } }), [running]);
  const actions = useMemo(() => {
    const api = createDesktopMockApi();
    const result = createMetaverseRoomActions({ api, activeTopic: 'kukuri:topic:demo', activeComposeChannel: { kind: 'public' }, onRefresh: async () => undefined });
    result.getHosting = async (context, id) => {
      if (unavailable) throw new Error('hosting read failed');
      const view = await api.getDomeHosting(context, id);
      return { ...view, state: { ...view.state, kind: running ? 'owner_hosted' : 'closed' } };
    };
    return result;
  }, [running, unavailable]);
  return <DomeManagementPanel room={room} actions={actions} endpointId='review-endpoint' locale='ja' admitted={false}
    onEnter={async () => false} onStopped={() => undefined} onDeleted={() => undefined} onClose={() => undefined} />;
}
const meta = { title: 'Extended/Dome Management', component: ManagementStory } satisfies Meta<typeof ManagementStory>;
export default meta;
type Story = StoryObj<typeof meta>;
export const StoppedOwner: Story = {};
export const RunningOwner: Story = { args: { running: true } };
export const ReadFailure: Story = { args: { unavailable: true } };
