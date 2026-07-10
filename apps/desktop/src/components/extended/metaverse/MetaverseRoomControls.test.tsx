import { createRef } from 'react';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, test, vi } from 'vitest';

import type { GameRoomView } from '@/lib/api';
import { MetaverseRoomControls } from './MetaverseRoomControls';

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

function renderControls(
  overrides: Partial<Parameters<typeof MetaverseRoomControls>[0]> = {}
) {
  const props: Parameters<typeof MetaverseRoomControls>[0] = {
    room,
    activeTopic: 'kukuri:topic:demo',
    localPeerId: 'local-peer',
    knownPeerCount: 2,
    lastSentSeq: 7,
    lastReceivedAt: null,
    remoteAnimationSummary: '',
    avatarAssetStatus: 'sample-vrm',
    localAvatarAssetRef: null,
    communityAssistAvailable: true,
    connectionState: 'live',
    locale: 'en',
    pending: false,
    hudOpen: true,
    hudDebugOpen: false,
    chatOpen: true,
    messages: [],
    messageDraft: '',
    messageInputRef: createRef<HTMLInputElement>(),
    onLeaveRoom: vi.fn(),
    onToggleHud: vi.fn(),
    onToggleHudDebug: vi.fn(),
    onImportAvatar: vi.fn(),
    onImportDefaultAvatar: vi.fn(),
    onMoveSharedObject: vi.fn(),
    onCloseChat: vi.fn(),
    onOpenChat: vi.fn(),
    onMessageDraftChange: vi.fn(),
    onSendMessage: vi.fn((event) => event.preventDefault()),
    ...overrides,
  };
  return { ...render(<MetaverseRoomControls {...props} />), props };
}

describe('MetaverseRoomControls', () => {
  test.each([
    ['live', 'Live', 'Room events are flowing'],
    ['recovering', 'Recovering', 'Refreshing room connectivity'],
    ['stale', 'Stale', 'No room activity recently'],
    ['offline', 'Offline', 'Peer connectivity is unavailable'],
  ] as const)('renders %s connection status and detail', (connectionState, label, detail) => {
    renderControls({ connectionState, hudOpen: false, chatOpen: false });

    expect(screen.getByText(label).closest('.metaverse-connection-badge')).toHaveAttribute(
      'title',
      detail
    );
  });

  test('routes toolbar, avatar, and shared-object controls through callbacks', async () => {
    const user = userEvent.setup();
    const onLeaveRoom = vi.fn();
    const onToggleHud = vi.fn();
    const onToggleHudDebug = vi.fn();
    const onImportAvatar = vi.fn();
    const onImportDefaultAvatar = vi.fn();
    const onMoveSharedObject = vi.fn();
    renderControls({
      hudDebugOpen: true,
      onLeaveRoom,
      onToggleHud,
      onToggleHudDebug,
      onImportAvatar,
      onImportDefaultAvatar,
      onMoveSharedObject,
    });

    expect(screen.getByText('Topic: kukuri:topic:demo')).toBeInTheDocument();
    expect(screen.getByText('Community assist: available')).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Leave room' }));
    await user.click(screen.getByRole('button', { name: 'Hide room HUD' }));
    await user.click(screen.getByRole('button', { name: 'Debug details' }));
    await user.upload(
      screen.getByLabelText('VRM file'),
      new File(['avatar'], 'avatar.vrm', { type: 'model/vrm' })
    );
    await user.click(screen.getByRole('button', { name: 'Default' }));
    for (const name of ['Forward', 'Left', 'Right', 'Back']) {
      await user.click(screen.getByRole('button', { name }));
    }

    expect(onLeaveRoom).toHaveBeenCalledTimes(1);
    expect(onToggleHud).toHaveBeenCalledTimes(1);
    expect(onToggleHudDebug).toHaveBeenCalledTimes(1);
    expect(onImportAvatar).toHaveBeenCalledWith(expect.objectContaining({ name: 'avatar.vrm' }));
    expect(onImportDefaultAvatar).toHaveBeenCalledTimes(1);
    expect(onMoveSharedObject.mock.calls).toEqual([
      [[0, 0, -50]],
      [[-50, 0, 0]],
      [[50, 0, 0]],
      [[0, 0, 50]],
    ]);
  });

  test('renders chat authors and routes draft, submit, and close actions', async () => {
    const user = userEvent.setup();
    const onMessageDraftChange = vi.fn();
    const onCloseChat = vi.fn();
    const onSendMessage = vi.fn((event: React.FormEvent<HTMLFormElement>) =>
      event.preventDefault()
    );
    renderControls({
      messageDraft: 'hello',
      messages: [
        {
          roomId: room.room_id,
          messageId: 'local-message',
          authorPeerId: 'local-peer',
          displayName: null,
          body: 'Local hello',
          createdAt: 1,
        },
        {
          roomId: room.room_id,
          messageId: 'remote-message',
          authorPeerId: 'remote-peer',
          displayName: 'Remote Friend',
          body: 'Remote hello',
          createdAt: 2,
        },
      ],
      onMessageDraftChange,
      onCloseChat,
      onSendMessage,
    });

    expect(screen.getByText('You')).toBeInTheDocument();
    expect(screen.getByText('Remote Friend')).toBeInTheDocument();
    const input = screen.getByLabelText('Room chat message');
    await user.type(input, '!');
    expect(onMessageDraftChange).toHaveBeenLastCalledWith('hello!');
    await user.click(screen.getByRole('button', { name: 'Send' }));
    await user.click(screen.getByRole('button', { name: 'Hide room chat' }));
    expect(onSendMessage).toHaveBeenCalledTimes(1);
    expect(onCloseChat).toHaveBeenCalledTimes(1);
  });

  test('shows the chat reopen action and disables avatar inputs while pending', async () => {
    const user = userEvent.setup();
    const onOpenChat = vi.fn();
    renderControls({ chatOpen: false, pending: true, onOpenChat });

    expect(screen.getByLabelText('VRM file')).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Default' })).toBeDisabled();
    await user.click(screen.getByRole('button', { name: 'Open room chat' }));
    expect(onOpenChat).toHaveBeenCalledTimes(1);
  });
});
