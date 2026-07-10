import {
  useEffect,
  useRef,
  useState,
  type FormEventHandler,
} from 'react';

import { Card } from '@/components/ui/card';
import type { SupportedLocale } from '@/i18n';
import type { GameRoomView, MetaverseAssetRef, SharedRoomObjectV1 } from '@/lib/api';
import { MetaverseScene } from '../MetaverseScene';
import type {
  AvatarAssetStatus,
  AvatarTransform,
  LatestChatBubble,
  MetaverseRoomConnectionState,
  MetaverseVec3,
  PeerPresence,
  RoomChatMessage,
} from '../MetaverseSceneModel';
import { MetaverseRoomControls } from './MetaverseRoomControls';

export type MetaverseRoomViewProps = {
  room: GameRoomView | null;
  activeTopic: string;
  localPeerId: string;
  remoteTransforms: Record<string, AvatarTransform>;
  peerPresence: Record<string, PeerPresence>;
  sharedObject: SharedRoomObjectV1;
  avatarAssetUrl: string | null;
  latestChatByPeer: Record<string, LatestChatBubble>;
  connectionState: MetaverseRoomConnectionState;
  now: number;
  knownPeerCount: number;
  lastSentSeq: number;
  lastReceivedAt: number | null;
  remoteAnimationSummary: string;
  avatarAssetStatus: AvatarAssetStatus;
  localAvatarAssetRef: MetaverseAssetRef | null;
  communityAssistAvailable: boolean;
  locale: SupportedLocale;
  pending: boolean;
  messages: RoomChatMessage[];
  messageDraft: string;
  initialHudOpen?: boolean;
  initialHudDebugOpen?: boolean;
  initialChatOpen?: boolean;
  onLocalTransform: (transform: AvatarTransform) => void;
  onAvatarAssetStatus: (status: AvatarAssetStatus) => void;
  onLeaveRoom: () => void;
  onImportAvatar: (file: File) => void;
  onImportDefaultAvatar: () => void;
  onMoveSharedObject: (delta: MetaverseVec3) => void;
  onMessageDraftChange: (value: string) => void;
  onSendMessage: FormEventHandler<HTMLFormElement>;
};

function isEditableTarget(target: EventTarget | null) {
  if (!(target instanceof HTMLElement)) {
    return false;
  }
  const tagName = target.tagName.toLowerCase();
  return tagName === 'input' || tagName === 'textarea' || tagName === 'select' || target.isContentEditable;
}

export function MetaverseRoomView({
  room,
  activeTopic,
  localPeerId,
  remoteTransforms,
  peerPresence,
  sharedObject,
  avatarAssetUrl,
  latestChatByPeer,
  connectionState,
  now,
  knownPeerCount,
  lastSentSeq,
  lastReceivedAt,
  remoteAnimationSummary,
  avatarAssetStatus,
  localAvatarAssetRef,
  communityAssistAvailable,
  locale,
  pending,
  messages,
  messageDraft,
  initialHudOpen = true,
  initialHudDebugOpen = false,
  initialChatOpen = true,
  onLocalTransform,
  onAvatarAssetStatus,
  onLeaveRoom,
  onImportAvatar,
  onImportDefaultAvatar,
  onMoveSharedObject,
  onMessageDraftChange,
  onSendMessage,
}: MetaverseRoomViewProps) {
  const [hudOpen, setHudOpen] = useState(initialHudOpen);
  const [hudDebugOpen, setHudDebugOpen] = useState(initialHudDebugOpen);
  const [chatOpen, setChatOpen] = useState(initialChatOpen);
  const messageInputRef = useRef<HTMLInputElement | null>(null);

  useEffect(() => {
    if (!room) {
      return;
    }
    let focusFrameId = 0;
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key !== 'Enter' || isEditableTarget(event.target)) {
        return;
      }
      event.preventDefault();
      setChatOpen(true);
      if (focusFrameId) {
        window.cancelAnimationFrame(focusFrameId);
      }
      focusFrameId = window.requestAnimationFrame(() => {
        messageInputRef.current?.focus();
      });
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => {
      window.removeEventListener('keydown', handleKeyDown);
      if (focusFrameId) {
        window.cancelAnimationFrame(focusFrameId);
      }
    };
  }, [room]);

  if (!room) {
    return null;
  }

  return (
    <Card className='shell-workspace-card metaverse-room-view'>
      <div className='metaverse-room-stage'>
        <MetaverseScene
          room={room}
          localPeerId={localPeerId}
          remoteTransforms={remoteTransforms}
          peerPresence={peerPresence}
          sharedObject={sharedObject}
          avatarAssetUrl={avatarAssetUrl}
          latestChatByPeer={latestChatByPeer}
          connectionState={connectionState}
          now={now}
          locale={locale}
          onLocalTransform={onLocalTransform}
          onAvatarAssetStatus={onAvatarAssetStatus}
          hud={(
            <MetaverseRoomControls
              room={room}
              activeTopic={activeTopic}
              localPeerId={localPeerId}
              knownPeerCount={knownPeerCount}
              lastSentSeq={lastSentSeq}
              lastReceivedAt={lastReceivedAt}
              remoteAnimationSummary={remoteAnimationSummary}
              avatarAssetStatus={avatarAssetStatus}
              localAvatarAssetRef={localAvatarAssetRef}
              communityAssistAvailable={communityAssistAvailable}
              connectionState={connectionState}
              locale={locale}
              pending={pending}
              hudOpen={hudOpen}
              hudDebugOpen={hudDebugOpen}
              chatOpen={chatOpen}
              messages={messages}
              messageDraft={messageDraft}
              messageInputRef={messageInputRef}
              onLeaveRoom={onLeaveRoom}
              onToggleHud={() => setHudOpen((open) => !open)}
              onToggleHudDebug={() => setHudDebugOpen((open) => !open)}
              onImportAvatar={onImportAvatar}
              onImportDefaultAvatar={onImportDefaultAvatar}
              onMoveSharedObject={onMoveSharedObject}
              onCloseChat={() => setChatOpen(false)}
              onOpenChat={() => setChatOpen(true)}
              onMessageDraftChange={onMessageDraftChange}
              onSendMessage={onSendMessage}
            />
          )}
        />
      </div>
    </Card>
  );
}
