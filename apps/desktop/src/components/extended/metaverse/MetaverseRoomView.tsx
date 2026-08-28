import {
  useEffect,
  useRef,
  useState,
  type FormEventHandler,
} from 'react';

import { Card } from '@/components/ui/card';
import type { SupportedLocale } from '@/i18n';
import type { DomeCustomizationV1, GameRoomView, MetaverseAssetRef, MetaverseInteractionKind, SharedRoomObjectV1 } from '@/lib/api';
import { MetaverseScene, type SessionPropView } from '../MetaverseScene';
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
import { useColumnRuntime } from '@/shell/ColumnRuntimeContext';

export type MetaverseRoomViewProps = {
  room: GameRoomView | null;
  activeTopic: string;
  localPeerId: string;
  remoteTransforms: Record<string, AvatarTransform>;
  peerPresence: Record<string, PeerPresence>;
  sharedObject: SharedRoomObjectV1;
  sessionProps?: SessionPropView[];
  avatarAssetUrl: string | null;
  domeTextureUrls: { wall: string | null; floor: string | null };
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
  isOwner: boolean;
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
  onSaveCustomization: (customization: DomeCustomizationV1) => Promise<void>;
  onImportTexture: (file: File) => Promise<MetaverseAssetRef>;
  onMoveSharedObject: (delta: MetaverseVec3) => void;
  onInteractWithProp: (interaction: MetaverseInteractionKind) => void;
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
  sessionProps,
  avatarAssetUrl,
  domeTextureUrls,
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
  isOwner,
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
  onSaveCustomization,
  onImportTexture,
  onMoveSharedObject,
  onInteractWithProp,
  onMessageDraftChange,
  onSendMessage,
}: MetaverseRoomViewProps) {
  const [hudOpen, setHudOpen] = useState(initialHudOpen);
  const [hudDebugOpen, setHudDebugOpen] = useState(initialHudDebugOpen);
  const [chatOpen, setChatOpen] = useState(initialChatOpen);
  const [sceneFocused, setSceneFocused] = useState(false);
  const messageInputRef = useRef<HTMLInputElement | null>(null);
  const stageRef = useRef<HTMLDivElement | null>(null);
  const runtime = useColumnRuntime();
  const controlsEnabled = runtime.active && runtime.visible && !runtime.suspended && sceneFocused;

  useEffect(() => {
    if (!room || !controlsEnabled) {
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
  }, [controlsEnabled, room]);

  if (!room) {
    return null;
  }

  return (
    <Card className='shell-workspace-card metaverse-room-view'>
      <div
        ref={stageRef}
        className='metaverse-room-stage'
        data-column-gesture-owner='metaverse'
        data-scene-focused={sceneFocused || undefined}
        tabIndex={0}
        onFocus={(event) => setSceneFocused(event.target === event.currentTarget)}
        onBlur={() => setSceneFocused(false)}
        onPointerDown={(event) => {
          if (
            event.target instanceof Element &&
            event.target.closest('button, input, textarea, select, a, [contenteditable="true"]')
          ) return;
          stageRef.current?.focus({ preventScroll: true });
        }}
      >
        <MetaverseScene
          room={room}
          localPeerId={localPeerId}
          remoteTransforms={remoteTransforms}
          peerPresence={peerPresence}
          sharedObject={sharedObject}
          sessionProps={sessionProps}
          avatarAssetUrl={avatarAssetUrl}
          domeTextureUrls={domeTextureUrls}
          latestChatByPeer={latestChatByPeer}
          connectionState={connectionState}
          now={now}
          locale={locale}
          onLocalTransform={onLocalTransform}
          onAvatarAssetStatus={onAvatarAssetStatus}
          controlsEnabled={controlsEnabled}
          suspended={runtime.suspended}
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
              isOwner={isOwner}
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
              onSaveCustomization={onSaveCustomization}
              onImportTexture={onImportTexture}
              onMoveSharedObject={onMoveSharedObject}
              onInteractWithProp={onInteractWithProp}
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
