import type { FormEventHandler, RefObject } from 'react';
import {
  Box,
  ChevronDown,
  LogOut,
  MessageSquare,
  MonitorPause,
  Move3D,
  PanelRightClose,
  PanelRightOpen,
  RefreshCw,
  Send,
  Wifi,
  WifiOff,
  X,
} from 'lucide-react';

import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import type { SupportedLocale } from '@/i18n';
import { formatLocalizedTime } from '@/i18n/format';
import type { GameRoomView, MetaverseAssetRef } from '@/lib/api';
import type {
  AvatarAssetStatus,
  MetaverseRoomConnectionState,
  MetaverseVec3,
  RoomChatMessage,
} from '../MetaverseSceneModel';

type MetaverseRoomControlsProps = {
  room: GameRoomView;
  activeTopic: string;
  localPeerId: string;
  knownPeerCount: number;
  lastSentSeq: number;
  lastReceivedAt: number | null;
  remoteAnimationSummary: string;
  avatarAssetStatus: AvatarAssetStatus;
  localAvatarAssetRef: MetaverseAssetRef | null;
  communityAssistAvailable: boolean;
  connectionState: MetaverseRoomConnectionState;
  locale: SupportedLocale;
  pending: boolean;
  hudOpen: boolean;
  hudDebugOpen: boolean;
  chatOpen: boolean;
  messages: RoomChatMessage[];
  messageDraft: string;
  messageInputRef: RefObject<HTMLInputElement | null>;
  onLeaveRoom: () => void;
  onToggleHud: () => void;
  onToggleHudDebug: () => void;
  onImportAvatar: (file: File) => void;
  onImportDefaultAvatar: () => void;
  onMoveSharedObject: (delta: MetaverseVec3) => void;
  onCloseChat: () => void;
  onOpenChat: () => void;
  onMessageDraftChange: (value: string) => void;
  onSendMessage: FormEventHandler<HTMLFormElement>;
};

function connectionStateLabel(state: MetaverseRoomConnectionState) {
  if (state === 'live') {
    return 'Live';
  }
  if (state === 'recovering') {
    return 'Recovering';
  }
  if (state === 'stale') {
    return 'Stale';
  }
  return 'Offline';
}

function connectionStateDetail(state: MetaverseRoomConnectionState) {
  if (state === 'live') {
    return 'Room events are flowing';
  }
  if (state === 'recovering') {
    return 'Refreshing room connectivity';
  }
  if (state === 'stale') {
    return 'No room activity recently';
  }
  return 'Peer connectivity is unavailable';
}

function ConnectionStateIcon({ state }: { state: MetaverseRoomConnectionState }) {
  if (state === 'live') {
    return <Wifi className='size-4' aria-hidden='true' />;
  }
  if (state === 'recovering') {
    return <RefreshCw className='size-4' aria-hidden='true' />;
  }
  if (state === 'stale') {
    return <MonitorPause className='size-4' aria-hidden='true' />;
  }
  return <WifiOff className='size-4' aria-hidden='true' />;
}

export function MetaverseRoomControls({
  room,
  activeTopic,
  localPeerId,
  knownPeerCount,
  lastSentSeq,
  lastReceivedAt,
  remoteAnimationSummary,
  avatarAssetStatus,
  localAvatarAssetRef,
  communityAssistAvailable,
  connectionState,
  locale,
  pending,
  hudOpen,
  hudDebugOpen,
  chatOpen,
  messages,
  messageDraft,
  messageInputRef,
  onLeaveRoom,
  onToggleHud,
  onToggleHudDebug,
  onImportAvatar,
  onImportDefaultAvatar,
  onMoveSharedObject,
  onCloseChat,
  onOpenChat,
  onMessageDraftChange,
  onSendMessage,
}: MetaverseRoomControlsProps) {
  return (
    <>
      <div
        className='metaverse-connection-badge'
        data-state={connectionState}
        title={connectionStateDetail(connectionState)}
      >
        <ConnectionStateIcon state={connectionState} />
        <span>{connectionStateLabel(connectionState)}</span>
      </div>
      <div className='metaverse-hud-toolbar' data-open={hudOpen}>
        <Button
          variant='ghost'
          size='icon'
          className='metaverse-hud-icon-button'
          type='button'
          aria-label='Leave room'
          onClick={onLeaveRoom}
        >
          <LogOut className='size-4' aria-hidden='true' />
        </Button>
        <Button
          variant='ghost'
          size='icon'
          className='metaverse-hud-icon-button'
          type='button'
          aria-label={hudOpen ? 'Hide room HUD' : 'Open room HUD'}
          onClick={onToggleHud}
        >
          {hudOpen ? (
            <PanelRightClose className='size-4' aria-hidden='true' />
          ) : (
            <PanelRightOpen className='size-4' aria-hidden='true' />
          )}
        </Button>
      </div>
      {hudOpen ? (
        <>
          <aside className='metaverse-room-hud'>
            <div className='panel-header metaverse-hud-header'>
              <div>
                <h3>{room.title}</h3>
                <small>{room.room_id}</small>
              </div>
            </div>
            <section className='metaverse-hud-accordion' data-open={hudDebugOpen}>
              <button
                type='button'
                className='metaverse-hud-accordion-trigger'
                aria-expanded={hudDebugOpen}
                onClick={onToggleHudDebug}
              >
                <span>Debug details</span>
                <ChevronDown className='size-4' aria-hidden='true' />
              </button>
              {hudDebugOpen ? (
                <div className='metaverse-room-diagnostics'>
                  <span>Topic: {activeTopic}</span>
                  <span>Local peer: {localPeerId}</span>
                  <span>Known peers: {knownPeerCount}</span>
                  <span>Last sent seq: {lastSentSeq}</span>
                  <span>
                    Last received: {lastReceivedAt ? formatLocalizedTime(lastReceivedAt, locale) : 'none'}
                  </span>
                  <span>Remote animation: {remoteAnimationSummary || 'none'}</span>
                  <span>
                    Avatar asset:{' '}
                    {avatarAssetStatus === 'sample-vrm'
                      ? 'sample VRM loaded'
                      : avatarAssetStatus === 'blob-vrm'
                        ? 'blob VRM loaded'
                        : avatarAssetStatus}
                  </span>
                  <span>
                    Blob asset resolve:{' '}
                    {localAvatarAssetRef?.blob_hash ?? 'public sample / fallback-ready'}
                  </span>
                  <span>Persistence: manifest blob {room.manifest_blob_hash ?? 'pending'}</span>
                  <span>Community assist: {communityAssistAvailable ? 'available' : 'optional'}</span>
                </div>
              ) : null}
            </section>
            <div className='metaverse-object-controls'>
              <strong>Avatar asset</strong>
              <div className='metaverse-avatar-asset-controls'>
                <Label>
                  <span className='sr-only'>VRM file</span>
                  <Input
                    type='file'
                    accept='.vrm,model/vrm,application/octet-stream'
                    disabled={pending}
                    onChange={(event) => {
                      const file = event.target.files?.[0];
                      if (file) {
                        onImportAvatar(file);
                      }
                      event.currentTarget.value = '';
                    }}
                  />
                </Label>
                <Button
                  size='sm'
                  variant='secondary'
                  type='button'
                  disabled={pending}
                  onClick={onImportDefaultAvatar}
                >
                  Default
                </Button>
              </div>
            </div>
            <div className='metaverse-object-controls'>
              <strong>
                <Box className='size-4' aria-hidden='true' />
                Shared object
              </strong>
              <div className='metaverse-nudge-grid'>
                <Button
                  size='sm'
                  variant='secondary'
                  type='button'
                  onClick={() => onMoveSharedObject([0, 0, -50])}
                >
                  <Move3D className='size-4' aria-hidden='true' />
                  Forward
                </Button>
                <Button size='sm' variant='secondary' type='button' onClick={() => onMoveSharedObject([-50, 0, 0])}>
                  Left
                </Button>
                <Button size='sm' variant='secondary' type='button' onClick={() => onMoveSharedObject([50, 0, 0])}>
                  Right
                </Button>
                <Button size='sm' variant='secondary' type='button' onClick={() => onMoveSharedObject([0, 0, 50])}>
                  Back
                </Button>
              </div>
            </div>
          </aside>
          <span className='metaverse-hud-scrollbar-indicator' aria-hidden='true'>
            <span />
          </span>
        </>
      ) : null}
      {chatOpen ? (
        <section className='metaverse-room-chat-log' aria-label='ROOM Chat'>
          <div className='metaverse-room-chat-log-header'>
            <span>
              <MessageSquare className='size-4' aria-hidden='true' />
              ROOM Chat
            </span>
            <Button
              variant='ghost'
              size='icon'
              className='metaverse-chat-close-button'
              type='button'
              aria-label='Hide room chat'
              onClick={onCloseChat}
            >
              <X className='size-4' aria-hidden='true' />
            </Button>
          </div>
          <ul className='metaverse-chat-list'>
            {messages.map((message) => (
              <li key={message.messageId}>
                <strong>
                  {message.authorPeerId === localPeerId
                    ? 'You'
                    : message.displayName || message.authorPeerId.slice(0, 12)}
                  <small>{formatLocalizedTime(message.createdAt, locale)}</small>
                </strong>
                <span>{message.body}</span>
              </li>
            ))}
          </ul>
          <form className='metaverse-chat-form' onSubmit={onSendMessage}>
            <Label>
              <span className='sr-only'>Room chat message</span>
              <Input
                ref={messageInputRef}
                value={messageDraft}
                placeholder='Say something in the room'
                onChange={(event) => onMessageDraftChange(event.target.value)}
              />
            </Label>
            <Button size='sm' type='submit'>
              <Send className='size-4' aria-hidden='true' />
              Send
            </Button>
          </form>
        </section>
      ) : (
        <Button
          variant='secondary'
          size='icon'
          className='metaverse-chat-toggle'
          type='button'
          aria-label='Open room chat'
          onClick={onOpenChat}
        >
          <MessageSquare className='size-4' aria-hidden='true' />
        </Button>
      )}
    </>
  );
}
