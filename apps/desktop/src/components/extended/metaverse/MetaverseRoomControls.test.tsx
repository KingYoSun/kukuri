import { createRef } from 'react';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, test, vi } from 'vitest';

import i18n from '@/i18n';
import type { GameRoomView } from '@/lib/api';
import { MetaverseRoomControls } from './MetaverseRoomControls';
import { createDefaultMetaverseRoomState } from './DomeSceneModel';

afterEach(async () => {
  await i18n.changeLanguage('en');
});

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
    isOwner: true,
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
    onSaveCustomization: vi.fn(),
    onImportTexture: vi.fn(),
    onMoveSharedObject: vi.fn(),
    onInteractWithProp: vi.fn(),
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
    ['en', 'Live', 'Leave room', 'ROOM Chat'],
    ['ja', '接続中', 'ルームから退出', 'ルームチャット'],
    ['zh-CN', '在线', '离开房间', '房间聊天'],
  ] as const)('renders the main HUD/chat surface in %s', async (locale, state, leave, chat) => {
    await i18n.changeLanguage(locale);
    renderControls({ locale });

    expect(screen.getByText(state)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: leave })).toBeInTheDocument();
    expect(screen.getByLabelText(chat)).toBeInTheDocument();
  });

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
    const onInteractWithProp = vi.fn();
    renderControls({
      hudDebugOpen: true,
      onLeaveRoom,
      onToggleHud,
      onToggleHudDebug,
      onImportAvatar,
      onImportDefaultAvatar,
      onMoveSharedObject,
      onInteractWithProp,
    });

    expect(screen.getByText('Topic: demo')).toBeInTheDocument();
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
    for (const name of ['Grab', 'Throw', 'Push', 'Sit']) {
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
    expect(onInteractWithProp.mock.calls).toEqual([['grab'], ['throw'], ['push'], ['sit']]);
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
