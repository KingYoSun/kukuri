import {
  useEffect,
  useLayoutEffect,
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
import type { MetaverseCategory, MetaverseOverlay, MetaversePanels } from './MetaverseCategories';
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
  panels?: MetaversePanels;
  hostingRequest?: number;
  sessionIdentity?: string;
  initialHudOpen?: boolean;
  initialOverlay?: MetaverseOverlay;
  initialCategory?: MetaverseCategory;
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
  panels,
  hostingRequest = 0,
  sessionIdentity,
  initialHudOpen = false,
  initialOverlay,
  initialCategory,
  initialHudDebugOpen = false,
  initialChatOpen = false,
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
  const [overlay, setOverlay] = useState<MetaverseOverlay>(initialOverlay ?? (initialChatOpen ? 'chat' : initialHudOpen ? 'details' : 'closed'));
  const [category, setCategory] = useState<MetaverseCategory>(initialCategory ?? (initialHudDebugOpen ? 'diagnostics' : 'dome'));
  const hudOpen = overlay === 'details';
  const chatOpen = overlay === 'chat';
  const [sceneFocused, setSceneFocused] = useState(false);
  const messageInputRef = useRef<HTMLInputElement | null>(null);
  const stageRef = useRef<HTMLDivElement | null>(null);
  const runtime = useColumnRuntime();
  const eligible = Boolean(room) && runtime.active && runtime.visible && !runtime.suspended;
  const eligibleRef = useRef(eligible);
  useLayoutEffect(() => { eligibleRef.current = eligible; }, [eligible]);
  const cameraState = useRef(createAvatarCameraState());
  const { mode, start, release } = useMetaverseSceneInput(stageRef, eligible, `${room?.room_id ?? ''}:${room?.metaverse?.instance_generation ?? ''}`);
  const controlsEnabled = eligible && overlay === 'closed' && sceneFocused && (mode === 'locked' || mode === 'unavailable');
  const startScene = () => { setOverlay('closed'); start(); };
  const cameraCommand = (command: CameraCommand) => { if (eligible && overlay === 'closed') applyCameraCommand(cameraState.current, command); };
  const closeOverlay = () => { setOverlay('closed'); start(); };
  const openOverlay = (next: MetaverseOverlay) => { release(); setOverlay(next); };

  useEffect(() => {
    if (hostingRequest > 0) { release(); setCategory('hosting'); setOverlay('details'); }
  }, [hostingRequest, release]);

  useEffect(() => {
    if (!eligibleRef.current || overlay === 'closed') return;
    const frame = requestAnimationFrame(() => {
      if (overlay === 'chat') messageInputRef.current?.focus();
      if (overlay === 'categories') stageRef.current?.querySelector<HTMLElement>(`.metaverse-category-menu [data-category="${category}"]`)?.focus();
      if (overlay === 'details') stageRef.current?.querySelector<HTMLElement>('[role="tab"][aria-selected="true"]')?.focus();
    });
    return () => cancelAnimationFrame(frame);
  }, [overlay, category]);

  useEffect(() => {
    if (!room || !eligible) {
      return;
    }
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.defaultPrevented || event.isComposing || event.repeat || event.ctrlKey || event.metaKey || event.altKey) return;
      if (event.key === 'Escape' && (sceneFocused || (event.target instanceof Node && stageRef.current?.contains(event.target)))) {
        if (document.querySelector('[role="dialog"][data-state="open"]')) return;
        event.preventDefault();
        event.stopPropagation();
        release();
        setOverlay('closed');
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
      setOverlay(event.key === 'Enter' ? 'chat' : 'categories');
    };
    // Consume owned UI keys before the shell's window-level Escape cascade.
    // The window fallback also handles the canvas Pointer Lock path.
    const stage = stageRef.current;
    stage?.addEventListener('keydown', handleKeyDown);
    window.addEventListener('keydown', handleKeyDown);
    return () => {
      stage?.removeEventListener('keydown', handleKeyDown);
      window.removeEventListener('keydown', handleKeyDown);
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
              overlay={overlay}
              category={category}
              panels={panels}
              sessionIdentity={sessionIdentity ?? `${localPeerId}:${room.room_id}:${room.metaverse?.instance_generation}`}
              onSelectCategory={(value) => { setCategory(value); openOverlay('details'); }}
              onCategories={() => openOverlay('categories')}
              onCloseOverlay={closeOverlay}
              chatOpen={chatOpen}
              messages={messages}
              messageDraft={messageDraft}
              messageInputRef={messageInputRef}
              onLeaveRoom={onLeaveRoom}
              onReturnHome={onReturnHome}
              onToggleHud={() => openOverlay('categories')}
              onImportAvatar={onImportAvatar}
              onImportDefaultAvatar={onImportDefaultAvatar}
              onSaveCustomization={onSaveCustomization}
              onImportTexture={onImportTexture}
              onMoveSharedObject={onMoveSharedObject}
              onInteractWithProp={onInteractWithProp}
              onCloseChat={closeOverlay}
              onOpenChat={() => openOverlay('chat')}
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
