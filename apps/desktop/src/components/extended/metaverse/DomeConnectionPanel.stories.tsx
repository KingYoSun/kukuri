import type { Meta, StoryObj } from '@storybook/react-vite';
import { useMemo } from 'react';
import { createDesktopMockApi } from '@/mocks/desktopApiMock';
import { createMetaverseRoomActions } from '@/shell/actions/metaverse';
import type { DomeBoundaryStateV1 } from '@/lib/api';
import { DomeConnectionPanel } from './DomeConnectionPanel';
import { connectionFixtureRooms, connectionFixtureTopology } from './DomeConnectionFixtures';
import { connectionScope } from './useDomeConnections';

function ConnectionStory({ state = 'ready', boundary = 'ready', partial = false, visitor = false, narrow = false }: {
  state?: 'ready' | 'loading' | 'error' | 'empty'; boundary?: DomeBoundaryStateV1; partial?: boolean; visitor?: boolean; narrow?: boolean;
}) {
  const actions = useMemo(() => createMetaverseRoomActions({ api: createDesktopMockApi(), activeTopic: 'kukuri:topic:connections-review',
    activeComposeChannel: { kind: 'public' }, onRefresh: async () => undefined }), []);
  const room = connectionFixtureRooms[0]; const topology = connectionFixtureTopology();
  if (boundary === 'draining') topology.connections[0].record.status = 'draining';
  if (boundary === 'blocked' || boundary === 'closed') {
    topology.connections[0].record.status = 'revoked'; topology.connections[0].record.lifecycle_reason = boundary === 'blocked' ? 'owners_blocked' : 'owner_revoked';
    topology.resolution.topology.components = [{ root_instance_id: 'a', instance_ids: ['a'], connection_ids: [], coordinates_cm: { a: [0, 0, 0] } }];
    topology.resolution.topology.active_connection_ids = [];
  }
  if (state === 'empty') { topology.connections = []; topology.resolution.topology.components = []; }
  return <div style={{ maxWidth: narrow ? 300 : 760 }}><DomeConnectionPanel room={room} rooms={partial ? [room] : state === 'empty' ? [room] : connectionFixtureRooms}
    localAuthorPubkey={visitor ? 'visitor' : room.host_pubkey} locale='ja' actions={actions} boundaries={{ east: boundary }}
    connections={{ scope: connectionScope(room, room.host_pubkey), topology: state === 'loading' ? null : topology,
      status: state === 'empty' ? 'ready' : state, error: state === 'error' ? '接続情報の更新に失敗しました' : null, refresh: async () => undefined }} /></div>;
}
const meta = { title: 'Extended/Dome Connections', component: ConnectionStory } satisfies Meta<typeof ConnectionStory>;
export default meta;
type Story = StoryObj<typeof meta>;
export const Confirmed: Story = {};
export const Partial: Story = { args: { partial: true } };
export const Loading: Story = { args: { state: 'loading' } };
export const ReadFailure: Story = { args: { state: 'error' } };
export const NoCandidates: Story = { args: { state: 'empty' } };
export const Offline: Story = { args: { boundary: 'offline' } };
export const Draining: Story = { args: { boundary: 'draining' } };
export const Blocked: Story = { args: { boundary: 'blocked' } };
export const Closed: Story = { args: { boundary: 'closed' } };
export const Visitor: Story = { args: { visitor: true } };
export const Narrow: Story = { args: { narrow: true } };
