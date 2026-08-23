import type { ReactNode } from 'react';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, test, vi } from 'vitest';

import type { GameRoomView, SharedRoomObjectV1 } from '@/lib/api';
import { MetaverseRoomView } from './MetaverseRoomView';
import { ColumnRuntimeProvider } from '@/shell/ColumnRuntimeContext';

vi.mock('../MetaverseScene', () => ({
  MetaverseScene: (props: {
    room: GameRoomView;
    localPeerId: string;
    sharedObject: SharedRoomObjectV1;
    connectionState: string;
    hud: ReactNode;
    controlsEnabled?: boolean;
    suspended?: boolean;
  }) => (
    <div aria-label='Metaverse room viewport'>
      <span>{`Scene room: ${props.room.room_id}`}</span>
      <span>{`Scene peer: ${props.localPeerId}`}</span>
      <span>{`Scene object: ${props.sharedObject.object_id}`}</span>
      <span>{`Scene connection: ${props.connectionState}`}</span>
      <span>{`Scene controls: ${String(props.controlsEnabled)}`}</span>
      <span>{`Scene suspended: ${String(props.suspended)}`}</span>
      {props.hud}
    </div>
  ),
}));

const room: GameRoomView = {
  room_id: 'metaverse-room-1',
  host_pubkey: 'f'.repeat(64),
  title: 'Atrium',
  description: 'Small social space',
  status: 'Waiting',
  phase_label: 'metaverse-mvp',
  scores: [],
  room_kind: 'metaverse_room',
  metaverse: null,
  manifest_blob_hash: 'manifest-1',
  updated_at: 1,
  channel_id: null,
  audience_label: 'Public',
};

const sharedObject: SharedRoomObjectV1 = {
  object_id: 'shared-object-1',
  asset_ref: null,
  primitive_fallback: 'cube',
  position: [0, 0, 0],
  rotation: [0, 0, 0],
  scale: [100, 100, 100],
  updated_by: 'local-peer',
  updated_at: 1,
};

function viewProps(
  overrides: Partial<Parameters<typeof MetaverseRoomView>[0]> = {}
): Parameters<typeof MetaverseRoomView>[0] {
  return {
    room,
    activeTopic: 'kukuri:topic:demo',
    localPeerId: 'local-peer',
    remoteTransforms: {},
    peerPresence: {},
    sharedObject,
    avatarAssetUrl: null,
    latestChatByPeer: {},
    connectionState: 'live',
    now: 1,
    knownPeerCount: 0,
    lastSentSeq: 0,
    lastReceivedAt: null,
    remoteAnimationSummary: '',
    avatarAssetStatus: 'sample-vrm',
    localAvatarAssetRef: null,
    communityAssistAvailable: true,
    locale: 'en',
    pending: false,
    messages: [],
    messageDraft: '',
    onLocalTransform: vi.fn(),
    onAvatarAssetStatus: vi.fn(),
    onLeaveRoom: vi.fn(),
    onImportAvatar: vi.fn(),
    onImportDefaultAvatar: vi.fn(),
    onMoveSharedObject: vi.fn(),
    onMessageDraftChange: vi.fn(),
    onSendMessage: vi.fn((event) => event.preventDefault()),
    ...overrides,
  };
}

afterEach(() => {
  vi.restoreAllMocks();
});

describe('MetaverseRoomView', () => {
  test('passes room session state into the scene and composes HUD/chat controls', () => {
    render(<MetaverseRoomView {...viewProps()} />);

    expect(screen.getByText('Scene room: metaverse-room-1')).toBeInTheDocument();
    expect(screen.getByText('Scene peer: local-peer')).toBeInTheDocument();
    expect(screen.getByText('Scene object: shared-object-1')).toBeInTheDocument();
    expect(screen.getByText('Scene connection: live')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Hide room HUD' })).toBeInTheDocument();
    expect(screen.getByLabelText('ROOM Chat')).toBeInTheDocument();
  });

  test('opens chat and focuses its input when Enter is pressed outside an editable target', async () => {
    render(<MetaverseRoomView {...viewProps({ initialChatOpen: false })} />);
    expect(screen.getByRole('button', { name: 'Open room chat' })).toBeInTheDocument();

    screen.getByLabelText('Metaverse room viewport').parentElement?.focus();
    await screen.findByText('Scene controls: true');
    fireEvent.keyDown(window, { key: 'Enter' });

    const input = await screen.findByLabelText('Room chat message');
    await waitFor(() => expect(input).toHaveFocus());
  });

  test('does not capture Enter from an editable target', () => {
    render(<MetaverseRoomView {...viewProps({ initialChatOpen: false })} />);

    fireEvent.keyDown(screen.getByLabelText('VRM file'), { key: 'Enter' });

    expect(screen.getByRole('button', { name: 'Open room chat' })).toBeInTheDocument();
  });

  test('preserves HUD and chat state while the selected room is temporarily absent', async () => {
    const user = userEvent.setup();
    const props = viewProps();
    const view = render(<MetaverseRoomView {...props} />);
    await user.click(screen.getByRole('button', { name: 'Hide room HUD' }));
    await user.click(screen.getByRole('button', { name: 'Hide room chat' }));

    view.rerender(<MetaverseRoomView {...props} room={null} />);
    expect(screen.queryByLabelText('Metaverse room viewport')).not.toBeInTheDocument();
    view.rerender(<MetaverseRoomView {...props} />);

    expect(screen.getByRole('button', { name: 'Open room HUD' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Open room chat' })).toBeInTheDocument();
  });

  test('removes the key listener and cancels a pending focus frame on cleanup', async () => {
    const removeEventListener = vi.spyOn(window, 'removeEventListener');
    const requestAnimationFrame = vi
      .spyOn(window, 'requestAnimationFrame')
      .mockImplementation(() => 17);
    const cancelAnimationFrame = vi.spyOn(window, 'cancelAnimationFrame');
    const view = render(<MetaverseRoomView {...viewProps({ initialChatOpen: false })} />);

    screen.getByLabelText('Metaverse room viewport').parentElement?.focus();
    await screen.findByText('Scene controls: true');
    fireEvent.keyDown(window, { key: 'Enter' });
    view.unmount();

    expect(requestAnimationFrame).toHaveBeenCalledTimes(1);
    expect(cancelAnimationFrame).toHaveBeenCalledWith(17);
    expect(removeEventListener).toHaveBeenCalledWith('keydown', expect.any(Function));
  });

  test('suspends rendering and keyboard controls without removing room session UI', () => {
    render(
      <ColumnRuntimeProvider value={{
        visible: false,
        active: false,
        audioFocused: false,
        suspended: true,
        requestAudioFocus: vi.fn(),
        releaseAudioFocus: vi.fn(),
      }}>
        <MetaverseRoomView {...viewProps({ initialChatOpen: false })} />
      </ColumnRuntimeProvider>
    );

    expect(screen.getByText('Scene controls: false')).toBeInTheDocument();
    expect(screen.getByText('Scene suspended: true')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Open room chat' })).toBeInTheDocument();
    fireEvent.keyDown(window, { key: 'Enter' });
    expect(screen.queryByLabelText('Room chat message')).not.toBeInTheDocument();
  });
});
