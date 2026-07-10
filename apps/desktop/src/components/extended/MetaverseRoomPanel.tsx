import { useState } from 'react';

import type {
  AuthorSocialView,
  ChannelRef,
  DesktopApi,
  GameRoomView,
  MetaverseAssetRef,
  Profile,
  SyncStatus,
} from '@/lib/api';
import type { SupportedLocale } from '@/i18n';
import { blobToBase64 } from '@/lib/attachments';
import {
  MetaverseRoomDiscovery,
  type CreateMetaverseRoomInput,
} from './metaverse/MetaverseRoomDiscovery';
import { MetaverseRoomView } from './metaverse/MetaverseRoomView';
import { useMetaverseRoomSession } from './metaverse/useMetaverseRoomSession';
import {
  DEFAULT_AVATAR_ASSET_NAME,
  DEFAULT_AVATAR_ASSET_URL,
  type AvatarAssetStatus,
} from './MetaverseSceneModel';

type MetaverseRoomPanelProps = {
  api: DesktopApi;
  activeTopic: string;
  activeComposeChannel: ChannelRef;
  rooms: GameRoomView[];
  syncStatus: SyncStatus;
  locale: SupportedLocale;
  localProfile?: Profile | null;
  knownAuthorsByPubkey?: Record<string, AuthorSocialView>;
  mediaObjectUrls?: Record<string, string | null>;
  onRefresh: () => Promise<void>;
};

export function MetaverseRoomPanel({
  api,
  activeTopic,
  activeComposeChannel,
  rooms,
  syncStatus,
  locale,
  localProfile = null,
  knownAuthorsByPubkey = {},
  mediaObjectUrls = {},
  onRefresh,
}: MetaverseRoomPanelProps) {
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState(false);
  const [avatarAssetStatus, setAvatarAssetStatus] = useState<AvatarAssetStatus>('loading');
  const [localAvatarAssetRef, setLocalAvatarAssetRef] = useState<MetaverseAssetRef | null>(null);
  const [localAvatarAssetUrl, setLocalAvatarAssetUrl] = useState<string | null>(null);
  const localDisplayName = localProfile?.display_name?.trim() || localProfile?.name?.trim() || null;
  const session = useMetaverseRoomSession({
    api,
    activeTopic,
    rooms,
    syncStatus,
    localDisplayName,
    localAvatarAssetRef,
    localAvatarAssetUrl,
    onRefresh,
    onError: setError,
  });

  async function handleCreateRoom(input: CreateMetaverseRoomInput) {
    setPending(true);
    try {
      const roomId = await api.createMetaverseRoom(
        activeTopic,
        input.title,
        input.description,
        input.maxPeers,
        activeComposeChannel
      );
      setError(null);
      session.selectCreatedRoom(roomId);
      await onRefresh();
      return true;
    } catch (createError) {
      setError(createError instanceof Error ? createError.message : 'Failed to create metaverse room');
      return false;
    } finally {
      setPending(false);
    }
  }

  async function importAvatarBlob(blob: Blob, name: string) {
    if (!session.selectedRoom) {
      return;
    }
    setPending(true);
    try {
      const mime = blob.type || 'model/vrm';
      const dataBase64 = await blobToBase64(blob);
      const assetRef = await api.importMetaverseRoomAsset(
        activeTopic,
        session.selectedRoom.room_id,
        'vrm',
        mime,
        name,
        dataBase64
      );
      const resolvedUrl =
        (await api.getBlobPreviewUrl(assetRef.blob_hash, assetRef.mime_type ?? mime)) ??
        `data:${mime};base64,${dataBase64}`;
      setLocalAvatarAssetRef(assetRef);
      setLocalAvatarAssetUrl(resolvedUrl);
      setError(null);
    } catch (assetError) {
      setError(assetError instanceof Error ? assetError.message : 'Failed to import VRM avatar');
    } finally {
      setPending(false);
    }
  }

  async function handleSampleAvatarImport() {
    const response = await fetch(DEFAULT_AVATAR_ASSET_URL);
    if (!response.ok) {
      throw new Error(`sample VRM fetch failed: ${response.status}`);
    }
    await importAvatarBlob(await response.blob(), DEFAULT_AVATAR_ASSET_NAME);
  }

  return (
    <div className='metaverse-panel'>
      <MetaverseRoomDiscovery
        rooms={rooms}
        selectedRoomId={session.selectedRoomId}
        joinedRoomIds={session.joinedRoomIds}
        pending={pending}
        error={error}
        locale={locale}
        localAuthorPubkey={syncStatus.local_author_pubkey}
        localProfile={localProfile}
        knownAuthorsByPubkey={knownAuthorsByPubkey}
        mediaObjectUrls={mediaObjectUrls}
        onCreateRoom={handleCreateRoom}
        onJoinRoom={session.joinRoom}
      />

      <MetaverseRoomView
        room={session.selectedRoom}
        activeTopic={activeTopic}
        localPeerId={session.localPeerId}
        remoteTransforms={session.remoteTransforms}
        peerPresence={session.peerPresence}
        sharedObject={session.sharedObject}
        avatarAssetUrl={localAvatarAssetUrl}
        latestChatByPeer={session.latestChatByPeer}
        connectionState={session.roomConnectionState}
        now={session.clockNow}
        knownPeerCount={session.knownPeerCount}
        lastSentSeq={session.lastSentSeq}
        lastReceivedAt={session.lastReceivedAt}
        remoteAnimationSummary={session.remoteAnimationSummary}
        avatarAssetStatus={avatarAssetStatus}
        localAvatarAssetRef={localAvatarAssetRef}
        communityAssistAvailable={syncStatus.discovery.bootstrap_seed_peer_ids.length > 0}
        locale={locale}
        pending={pending}
        messages={session.messages}
        messageDraft={session.messageDraft}
        onLocalTransform={session.handleLocalTransform}
        onAvatarAssetStatus={setAvatarAssetStatus}
        onLeaveRoom={session.leaveRoom}
        onImportAvatar={(file) => void importAvatarBlob(file, file.name)}
        onImportDefaultAvatar={() => void handleSampleAvatarImport()}
        onMoveSharedObject={session.moveSharedObject}
        onMessageDraftChange={session.setMessageDraft}
        onSendMessage={session.handleSendMessage}
      />
    </div>
  );
}
