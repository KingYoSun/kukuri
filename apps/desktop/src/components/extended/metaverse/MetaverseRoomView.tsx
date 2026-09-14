import {
  useEffect,
  useRef,
  useState,
  type FormEventHandler,
} from 'react';

import { Card } from '@/components/ui/card';
import type { SupportedLocale } from '@/i18n';
import type {
  DomeBoundaryStateV1,
  DomeCustomizationV1,
  DomeDirection,
  GameRoomView,
  MetaverseAssetRef,
  MetaverseInteractionKind,
  SharedRoomObjectV1,
} from '@/lib/api';
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
import { ONLINE_DOME_RECOVERY, type DomeRecoveryStatus } from './useMetaverseRoomSession';
import { useColumnRuntime } from '@/shell/ColumnRuntimeContext';
import type { DomeNeighborTransitionView } from './DomeTransitionModel';
import { useMetaverseSceneInput } from './useMetaverseSceneInput';
import { applyCameraCommand, createAvatarCameraState, type CameraCommand } from './MetaverseCameraModel';
import { MetaverseCameraControls } from './MetaverseCameraControls';

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
  transitionNeighbors?: DomeNeighborTransitionView[];
  transitionBoundaryStates?: Partial<Record<DomeDirection, DomeBoundaryStateV1>>;
  handoffTransform?: AvatarTransform | null;
  latestChatByPeer: Record<string, LatestChatBubble>;
  connectionState: MetaverseRoomConnectionState;
  domeRecovery?: DomeRecoveryStatus;
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
  onReturnHome?: () => void;
  onImportAvatar: (file: File) => void;
  onImportDefaultAvatar: () => void;
  onSaveCustomization: (customization: DomeCustomizationV1) => Promise<void>;
  onImportTexture: (file: File) => Promise<MetaverseAssetRef>;
  onMoveSharedObject: (delta: MetaverseVec3) => void;
  onInteractWithProp: (interaction: MetaverseInteractionKind) => void;
  onMessageDraftChange: (value: string) => void;
  onSendMessage: FormEventHandler<HTMLFormElement>;
  microphoneEnabled?: boolean;
  onToggleMicrophone?: () => void;
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
  transitionNeighbors,
  transitionBoundaryStates,
  handoffTransform,
  latestChatByPeer,
  connectionState,
  domeRecovery = ONLINE_DOME_RECOVERY,
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
  onReturnHome,
  onImportAvatar,
  onImportDefaultAvatar,
  onSaveCustomization,
  onImportTexture,
  onMoveSharedObject,
  onInteractWithProp,
  onMessageDraftChange,
  onSendMessage,
  microphoneEnabled = false,
  onToggleMicrophone,
}: MetaverseRoomViewProps) {
  const [hudOpen, setHudOpen] = useState(initialHudOpen);
  const [hudDebugOpen, setHudDebugOpen] = useState(initialHudDebugOpen);
  const [chatOpen, setChatOpen] = useState(initialChatOpen);
  const [sceneFocused, setSceneFocused] = useState(false);
  const messageInputRef = useRef<HTMLInputElement | null>(null);
  const stageRef = useRef<HTMLDivElement | null>(null);
  const runtime = useColumnRuntime();
  const eligible = Boolean(room) && runtime.active && runtime.visible && !runtime.suspended;
  const cameraState = useRef(createAvatarCameraState());
  const { mode, start, release } = useMetaverseSceneInput(stageRef, eligible, `${room?.room_id ?? ''}:${room?.metaverse?.instance_generation ?? ''}`);
  const controlsEnabled = eligible && sceneFocused && (mode === 'locked' || mode === 'unavailable');
  const startScene = () => { setHudOpen(false); setChatOpen(false); start(); };
  const cameraCommand = (command: CameraCommand) => { if (eligible) applyCameraCommand(cameraState.current, command); };
  const closeChat = () => {
    setChatOpen(false);
    if (!hudOpen) start();
    else release();
  };
  const toggleHud = () => {
    setHudOpen(!hudOpen);
    if (hudOpen && !chatOpen) start();
    else release();
  };

  useEffect(() => {
    if (!room || !eligible) {
      return;
    }
    let focusFrameId = 0;
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.isComposing || event.repeat || event.ctrlKey || event.metaKey || event.altKey) return;
      if (event.key === 'Escape' && (sceneFocused || stageRef.current?.contains(event.target as Node))) {
        release();
        setChatOpen(false);
        setHudOpen(false);
        stageRef.current?.focus({ preventScroll: true });
        return;
      }
      if (!sceneFocused || isEditableTarget(event.target)) return;
      if (event.key.toLowerCase() === 'r') {
        event.preventDefault();
        applyCameraCommand(cameraState.current, 'reset');
        return;
      }
      if (event.key !== 'Enter' && event.key !== 'Tab') {
        return;
      }
      event.preventDefault();
      release();
      const chat = event.key === 'Enter';
      setChatOpen(chat);
      setHudOpen(!chat);
      if (focusFrameId) {
        window.cancelAnimationFrame(focusFrameId);
      }
      focusFrameId = window.requestAnimationFrame(() => {
        if (chat) messageInputRef.current?.focus();
        else stageRef.current?.querySelector<HTMLElement>('.metaverse-room-hud button')?.focus();
      });
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => {
      window.removeEventListener('keydown', handleKeyDown);
      if (focusFrameId) {
        window.cancelAnimationFrame(focusFrameId);
      }
    };
  }, [eligible, release, room, sceneFocused]);

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
        data-input-mode={eligible ? mode : 'inactive'}
        tabIndex={0}
        onFocus={(event) => setSceneFocused(event.target === event.currentTarget)}
        onBlur={() => { setSceneFocused(false); release(); }}
        onPointerDown={(event) => {
          if (
            event.target instanceof Element &&
            event.target.closest('[data-metaverse-ui], button, input, textarea, select, a, [contenteditable="true"]')
          ) return;
          stageRef.current?.focus({ preventScroll: true });
        }}
        onClick={(event) => {
          if (event.target instanceof Element &&
            !event.target.closest('[data-metaverse-ui], button, input, textarea, select, a, [contenteditable="true"]') &&
            eligible && mode !== 'locked') startScene();
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
          transitionNeighbors={transitionNeighbors}
          transitionBoundaryStates={transitionBoundaryStates}
          initialLocalTransform={handoffTransform}
          latestChatByPeer={latestChatByPeer}
          connectionState={connectionState}
          now={now}
          locale={locale}
          onLocalTransform={onLocalTransform}
          onAvatarAssetStatus={onAvatarAssetStatus}
          controlsEnabled={controlsEnabled}
          cameraState={cameraState}
          suspended={runtime.suspended}
          hud={(
            <>
            {mode === 'locked' && <span className='metaverse-camera-reticle' aria-hidden='true'>+</span>}
            <MetaverseCameraControls locale={locale} mode={mode} enabled={eligible} onStart={startScene} onCommand={cameraCommand} />
            <div className='metaverse-ui-layer' data-metaverse-ui>
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
              domeRecovery={domeRecovery}
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
              onReturnHome={onReturnHome}
              onToggleHud={toggleHud}
              onToggleHudDebug={() => setHudDebugOpen((open) => !open)}
              onImportAvatar={onImportAvatar}
              onImportDefaultAvatar={onImportDefaultAvatar}
              onSaveCustomization={onSaveCustomization}
              onImportTexture={onImportTexture}
              onMoveSharedObject={onMoveSharedObject}
              onInteractWithProp={onInteractWithProp}
              onCloseChat={closeChat}
              onOpenChat={() => { release(); setChatOpen(true); }}
              onMessageDraftChange={onMessageDraftChange}
              onSendMessage={onSendMessage}
              microphoneEnabled={microphoneEnabled}
              onToggleMicrophone={onToggleMicrophone}
            />
            </div>
            </>
          )}
        />
      </div>
    </Card>
  );
}
