import type { Meta, StoryObj } from '@storybook/react-vite';

import type { GameRoomView, SyncStatus } from '@/lib/api';
import { createDesktopMockApi } from '@/mocks/desktopApiMock';
import { MetaverseRoomPanel } from './MetaverseRoomPanel';
import { DEFAULT_SHARED_OBJECT } from './MetaverseSceneModel';
import { MetaverseRoomDiscovery } from './metaverse/MetaverseRoomDiscovery';
import { MetaverseRoomView } from './metaverse/MetaverseRoomView';
import { createMetaverseRoomActions } from '@/shell/actions/metaverse';

const meta = {
  title: 'Extended/MetaverseRoomPanel',
  parameters: {
    layout: 'fullscreen',
  },
} satisfies Meta;

export default meta;

type Story = StoryObj<typeof meta>;

const STORY_TIMESTAMP = 1_742_860_800_000;
const LOCAL_PUBKEY = 'f'.repeat(64);

const room: GameRoomView = {
  room_id: 'metaverse-room-1',
  host_pubkey: LOCAL_PUBKEY,
  title: 'Atrium',
  description: 'Small social space',
  status: 'Waiting',
  phase_label: 'metaverse-mvp',
  scores: [],
  room_kind: 'metaverse_room',
  metaverse: {
    world_version: 1,
    max_peers: 8,
    scene: {
      ground: 'default',
      shared_object: DEFAULT_SHARED_OBJECT,
    },
    default_spawn: {
      position: [0, 0, 260],
      rotation: [0, 180, 0],
    },
    asset_refs: [],
    chat_history: [],
  },
  manifest_blob_hash: 'mock-metaverse-room-1',
  updated_at: STORY_TIMESTAMP,
  channel_id: null,
  audience_label: 'Public',
};

const syncStatus: SyncStatus = {
  connected: true,
  delivery_state: 'Live',
  peer_count: 1,
  pending_events: 0,
  status_detail: 'connected',
  configured_peers: [],
  subscribed_topics: ['kukuri:topic:demo'],
  active_path: 'direct_p2p',
  fallback_peer_ids: [],
  topic_diagnostics: [],
  local_author_pubkey: LOCAL_PUBKEY,
  discovery: {
    mode: 'seeded_dht',
    connect_mode: 'direct_only',
    active_path: 'direct_p2p',
    fallback_peer_ids: [],
    env_locked: false,
    configured_seed_peer_ids: [],
    bootstrap_seed_peer_ids: ['community-node'],
    manual_ticket_peer_ids: [],
    connected_peer_ids: [],
    docs_assist_peer_ids: [],
    blob_assist_peer_ids: [],
    local_endpoint_id: 'local-endpoint-a',
  },
  gossip_disabled_topics: [],
  gossip_disabled_channels: [],
};

function StoryFrame({ children }: { children: React.ReactNode }) {
  return (
    <div style={{ maxWidth: 1180, margin: '0 auto', padding: 24 }}>
      <div className='shell-main-stack'>{children}</div>
    </div>
  );
}

function panel(rooms: GameRoomView[]) {
  const api = createDesktopMockApi();
  return (
    <StoryFrame>
      <MetaverseRoomPanel
        actions={createMetaverseRoomActions({
          api,
          activeTopic: 'kukuri:topic:demo',
          activeComposeChannel: { kind: 'public' },
          onRefresh: async () => undefined,
        })}
        activeTopic='kukuri:topic:demo'
        rooms={rooms}
        syncStatus={syncStatus}
        locale='en'
        localProfile={{
          pubkey: LOCAL_PUBKEY,
          name: 'host',
          display_name: 'Host Author',
          about: null,
          picture: null,
          picture_asset: null,
          updated_at: STORY_TIMESTAMP,
        }}
      />
    </StoryFrame>
  );
}

function selectedRoom(initialHudOpen = true, initialChatOpen = true) {
  return (
    <StoryFrame>
      <MetaverseRoomView
        room={room}
        activeTopic='kukuri:topic:demo'
        localPeerId='local-endpoint-a:story'
        remoteTransforms={{}}
        peerPresence={{}}
        sharedObject={DEFAULT_SHARED_OBJECT}
        avatarAssetUrl={null}
        latestChatByPeer={{}}
        connectionState='live'
        now={STORY_TIMESTAMP}
        knownPeerCount={2}
        lastSentSeq={12}
        lastReceivedAt={STORY_TIMESTAMP - 1_000}
        remoteAnimationSummary='remote-pe:walk'
        avatarAssetStatus='sample-vrm'
        localAvatarAssetRef={null}
        communityAssistAvailable={true}
        locale='en'
        pending={false}
        messages={[
          {
            roomId: room.room_id,
            messageId: 'story-message-1',
            authorPeerId: 'remote-peer',
            displayName: 'Remote Friend',
            body: 'Welcome to the room.',
            createdAt: STORY_TIMESTAMP - 2_000,
          },
        ]}
        messageDraft=''
        initialHudOpen={initialHudOpen}
        initialHudDebugOpen={initialHudOpen}
        initialChatOpen={initialChatOpen}
        onLocalTransform={() => undefined}
        onAvatarAssetStatus={() => undefined}
        onLeaveRoom={() => undefined}
        onImportAvatar={() => undefined}
        onImportDefaultAvatar={() => undefined}
        onMoveSharedObject={() => undefined}
        onMessageDraftChange={() => undefined}
        onSendMessage={(event) => event.preventDefault()}
      />
    </StoryFrame>
  );
}

export const EmptyRooms: Story = {
  render: () => panel([]),
};

export const RoomList: Story = {
  render: () => panel([room]),
};

export const DiscoveryError: Story = {
  render: () => (
    <StoryFrame>
      <MetaverseRoomDiscovery
        rooms={[room]}
        selectedRoomId={null}
        joinedRoomIds={new Set()}
        pending={false}
        error='Room connectivity is unavailable'
        locale='en'
        localAuthorPubkey={LOCAL_PUBKEY}
        localProfile={null}
        knownAuthorsByPubkey={{}}
        mediaObjectUrls={{}}
        onCreateRoom={async () => false}
        onJoinRoom={() => undefined}
      />
    </StoryFrame>
  ),
};

export const SelectedHudAndChat: Story = {
  render: () => selectedRoom(),
};

export const SelectedCollapsed: Story = {
  render: () => selectedRoom(false, false),
};
