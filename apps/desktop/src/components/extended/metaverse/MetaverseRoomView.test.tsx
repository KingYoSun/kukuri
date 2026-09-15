import type { ReactNode } from 'react';
import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, test, vi } from 'vitest';

import type { GameRoomView, SharedRoomObjectV1 } from '@/lib/api';
import { MetaverseRoomView } from './MetaverseRoomView';
import { createDefaultMetaverseRoomState } from './DomeSceneModel';
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
  metaverse: createDefaultMetaverseRoomState(8),
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
    domeTextureUrls: { wall: null, floor: null },
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
    isOwner: true,
    messages: [],
    messageDraft: '',
    onLocalTransform: vi.fn(),
    onAvatarAssetStatus: vi.fn(),
    onLeaveRoom: vi.fn(),
    onImportAvatar: vi.fn(),
    onImportDefaultAvatar: vi.fn(),
    onSaveCustomization: vi.fn(),
    onImportTexture: vi.fn(),
    onMoveSharedObject: vi.fn(),
    onInteractWithProp: vi.fn(),
    onMessageDraftChange: vi.fn(),
    onSendMessage: vi.fn((event) => event.preventDefault()),
    ...overrides,
  };
}

afterEach(() => {
  vi.restoreAllMocks();
});

describe('MetaverseRoomView', () => {
  test('starts with only discoverable chat and menu entries', () => {
    render(<MetaverseRoomView {...viewProps()} />);
    expect(screen.queryByLabelText('ROOM Chat')).not.toBeInTheDocument();
    expect(screen.getByLabelText('Wall material')).not.toBeVisible();
    expect(screen.getByRole('button', { name: 'Menu (Tab)' })).toBeVisible();
  });

  test('category navigation and closing preserve an unsaved Dome draft without actions', async () => {
    const props = viewProps();
    const user = userEvent.setup();
    const view = render(<MetaverseRoomView {...props} />);
    await user.click(screen.getByRole('button', { name: 'Menu (Tab)' }));
    await user.click(screen.getByRole('button', { name: 'Dome settings' }));
    await user.selectOptions(screen.getByLabelText('Wall material'), 'wood');
    await user.click(screen.getByRole('tab', { name: 'Avatar' }));
    await user.click(screen.getByRole('tab', { name: 'Dome settings' }));
    expect(screen.getByLabelText('Wall material')).toHaveValue('wood');
    fireEvent.keyDown(document.activeElement!, { key: 'Escape' });
    expect(view.container.querySelector('.metaverse-room-stage')).toHaveFocus();
    await user.click(screen.getByRole('button', { name: 'Menu (Tab)' }));
    await user.click(screen.getByRole('button', { name: 'Dome settings' }));
    expect(screen.getByLabelText('Wall material')).toHaveValue('wood');
    expect(props.onSaveCustomization).not.toHaveBeenCalled();
    expect(props.onImportAvatar).not.toHaveBeenCalled();
    expect(props.onLocalTransform).not.toHaveBeenCalled();
  });

  test('owned Escape is consumed before window navigation and a prevented Escape stays with its control', async () => {
    const user = userEvent.setup();
    render(<MetaverseRoomView {...viewProps()} />);
    await user.click(screen.getByRole('button', { name: 'Menu (Tab)' }));
    const target = screen.getByRole('button', { name: 'Dome settings' });
    const consumed = new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true });
    consumed.preventDefault();
    fireEvent(target, consumed);
    expect(target).toBeVisible();
    const shell = vi.fn();
    window.addEventListener('keydown', shell);
    fireEvent.keyDown(target, { key: 'Escape' });
    expect(shell).not.toHaveBeenCalled();
    expect(target).not.toBeVisible();
    window.removeEventListener('keydown', shell);
  });
  test('does not register document input ownership before a room is rendered', () => {
    const listen = vi.spyOn(document, 'addEventListener');
    render(<MetaverseRoomView {...viewProps({ room: null })} />);
    expect(listen).not.toHaveBeenCalledWith('pointerlockchange', expect.any(Function));
  });
  test('provides explicit camera control and a pointer lock resume action', () => {
    render(<MetaverseRoomView {...viewProps()} />);
    expect(screen.getByRole('button', { name: 'Resume avatar controls' })).toBeEnabled();
    fireEvent.click(screen.getByText('Adjust view'));
    expect(screen.getByRole('button', { name: 'Reset camera' })).toBeEnabled();
  });

  test('clicking the 3D view enters avatar controls without using the resume button', () => {
    vi.spyOn(document, 'hasFocus').mockReturnValue(true);
    render(<MetaverseRoomView {...viewProps()} />);
    fireEvent.click(screen.getByLabelText('Metaverse room viewport'));
    expect(screen.queryByLabelText('ROOM Chat')).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Hide room HUD' })).not.toBeInTheDocument();
    // This mock has no canvas: explicit capture failure remains usable and visible.
    expect(screen.getByText('Pointer capture unavailable. Use the view adjustment buttons or resume to retry.')).toBeInTheDocument();
  });

  test('passes room session state into the scene and composes HUD/chat controls', () => {
    render(<MetaverseRoomView {...viewProps()} />);

    expect(screen.getByText('Scene room: metaverse-room-1')).toBeInTheDocument();
    expect(screen.getByText('Scene peer: local-peer')).toBeInTheDocument();
    expect(screen.getByText('Scene object: shared-object-1')).toBeInTheDocument();
    expect(screen.getByText('Scene connection: live')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Menu (Tab)' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Open room chat' })).toBeInTheDocument();
  });

  test('opens chat and focuses its input when Enter is pressed outside an editable target', async () => {
    render(<MetaverseRoomView {...viewProps({ initialChatOpen: false })} />);
    expect(screen.getByRole('button', { name: 'Open room chat' })).toBeInTheDocument();

    screen.getByLabelText('Metaverse room viewport').parentElement?.focus();
    await screen.findByText('Scene controls: false');
    fireEvent.keyDown(window, { key: 'Enter' });

    const input = await screen.findByLabelText('Room chat message');
    await waitFor(() => expect(input).toHaveFocus());
  });

  test('does not capture Enter from an editable target', () => {
    render(<MetaverseRoomView {...viewProps({ initialChatOpen: false })} />);

    fireEvent.keyDown(screen.getByLabelText('VRM file'), { key: 'Enter' });

    expect(screen.getByRole('button', { name: 'Open room chat' })).toBeInTheDocument();
  });

  test('Tab opens the HUD, normal UI Tab stays native and Escape restores scene focus', async () => {
    const user = userEvent.setup();
    const view = render(<MetaverseRoomView {...viewProps({ initialHudOpen: false, initialChatOpen: false })} />);
    const stage = view.container.querySelector<HTMLElement>('[data-column-gesture-owner]')!;
    act(() => stage.focus());
    fireEvent.keyDown(window, { key: 'Tab' });
    const debug = await screen.findByRole('button', { name: 'Dome settings' });
    await waitFor(() => expect(debug).toHaveFocus());
    await user.tab();
    expect(debug).not.toHaveFocus();
    fireEvent.keyDown(document.activeElement!, { key: 'Escape' });
    expect(stage).toHaveFocus();
    expect(screen.queryByRole('button', { name: 'Dome settings' })).not.toBeInTheDocument();
  });

  test('chat Escape keeps the draft, ignores IME confirmation and causes no domain actions', async () => {
    const props = viewProps({ initialHudOpen: false, initialChatOpen: false, messageDraft: 'unsent draft' });
    const view = render(<MetaverseRoomView {...props} />);
    const stage = view.container.querySelector<HTMLElement>('[data-column-gesture-owner]')!;
    act(() => stage.focus());
    fireEvent.keyDown(window, { key: 'Enter', isComposing: true });
    expect(screen.queryByLabelText('Room chat message')).not.toBeInTheDocument();
    fireEvent.keyDown(window, { key: 'Enter' });
    const input = await screen.findByLabelText('Room chat message');
    await waitFor(() => expect(input).toHaveFocus());
    fireEvent.keyDown(input, { key: 'Escape', isComposing: true });
    expect(input).toBeInTheDocument();
    fireEvent.keyDown(input, { key: 'Escape' });
    expect(stage).toHaveFocus();
    fireEvent.keyDown(window, { key: 'Enter' });
    expect(await screen.findByLabelText('Room chat message')).toHaveValue('unsent draft');
    expect(props.onSendMessage).not.toHaveBeenCalled();
    expect(props.onLocalTransform).not.toHaveBeenCalled();
    expect(props.onSaveCustomization).not.toHaveBeenCalled();
  });

  test('preserves HUD and chat state while the selected room is temporarily absent', async () => {
    const user = userEvent.setup();
    const props = viewProps();
    const view = render(<MetaverseRoomView {...props} />);
    await user.click(screen.getByRole('button', { name: 'Menu (Tab)' }));
    await user.click(screen.getByRole('button', { name: 'Close' }));

    view.rerender(<MetaverseRoomView {...props} room={null} />);
    expect(screen.queryByLabelText('Metaverse room viewport')).not.toBeInTheDocument();
    view.rerender(<MetaverseRoomView {...props} />);

    expect(screen.getByRole('button', { name: 'Menu (Tab)' })).toBeInTheDocument();
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
    await screen.findByText('Scene controls: false');
    fireEvent.keyDown(window, { key: 'Enter' });
    view.unmount();

    expect(requestAnimationFrame).toHaveBeenCalled();
    expect(cancelAnimationFrame).toHaveBeenCalledWith(17);
    expect(removeEventListener).toHaveBeenCalledWith('keydown', expect.any(Function));
  });

  test('a queued chat focus does not steal focus after the Column becomes inactive', () => {
    const callbacks: FrameRequestCallback[] = [];
    vi.spyOn(window, 'requestAnimationFrame').mockImplementation(callback => { callbacks.push(callback); return callbacks.length; });
    const props = viewProps({ initialChatOpen: true });
    const active = { visible: true, active: true, audioFocused: false, suspended: false, requestAudioFocus: vi.fn(), releaseAudioFocus: vi.fn() };
    const view = render(<ColumnRuntimeProvider value={active}><MetaverseRoomView {...props} /></ColumnRuntimeProvider>);
    view.rerender(<ColumnRuntimeProvider value={{ ...active, active: false, visible: false, suspended: true }}><MetaverseRoomView {...props} /></ColumnRuntimeProvider>);
    act(() => callbacks.forEach(callback => callback(0)));
    expect(screen.getByLabelText('Room chat message')).not.toHaveFocus();
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
