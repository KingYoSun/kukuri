import { useState } from 'react';
import { useTranslation } from 'react-i18next';

import type {
  AuthorSocialView,
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
import type { MetaverseRoomActions } from './metaverse/MetaverseRoomActions';
import {
  DEFAULT_AVATAR_ASSET_NAME,
  DEFAULT_AVATAR_ASSET_URL,
  type AvatarAssetStatus,
} from './MetaverseSceneModel';

type MetaverseRoomPanelProps = {
  actions: MetaverseRoomActions;
  activeTopic: string;
  rooms: GameRoomView[];
  syncStatus: SyncStatus;
  locale: SupportedLocale;
  localProfile?: Profile | null;
  knownAuthorsByPubkey?: Record<string, AuthorSocialView>;
  mediaObjectUrls?: Record<string, string | null>;
  initialSelectedRoomId?: string | null;
};

export function MetaverseRoomPanel({
  actions,
  activeTopic,
  rooms,
  syncStatus,
  locale,
  localProfile = null,
  knownAuthorsByPubkey = {},
  mediaObjectUrls = {},
  initialSelectedRoomId = null,
}: MetaverseRoomPanelProps) {
  const { t } = useTranslation('metaverse', { lng: locale });
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState(false);
  const [avatarAssetStatus, setAvatarAssetStatus] = useState<AvatarAssetStatus>('loading');
  const [localAvatarAssetRef, setLocalAvatarAssetRef] = useState<MetaverseAssetRef | null>(null);
  const [localAvatarAssetUrl, setLocalAvatarAssetUrl] = useState<string | null>(null);
  const localDisplayName = localProfile?.display_name?.trim() || localProfile?.name?.trim() || null;
  const session = useMetaverseRoomSession({
    actions,
    activeTopic,
    rooms,
    syncStatus,
    locale,
    localDisplayName,
    localAvatarAssetRef,
    localAvatarAssetUrl,
    initialSelectedRoomId,
    onError: setError,
  });

  async function handleCreateRoom(input: CreateMetaverseRoomInput) {
    setPending(true);
    try {
      const roomId = await actions.createRoom(input);
      setError(null);
      session.selectCreatedRoom(roomId);
      await actions.refresh();
      return true;
    } catch (createError) {
      setError(createError instanceof Error ? createError.message : t('errors.createFailed'));
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
      const assetRef = await actions.importRoomAsset(
        session.selectedRoom.room_id,
        'vrm',
        mime,
        name,
        dataBase64
      );
      const resolvedUrl =
        (await actions.getBlobPreviewUrl(assetRef.blob_hash, assetRef.mime_type ?? mime)) ??
        `data:${mime};base64,${dataBase64}`;
      setLocalAvatarAssetRef(assetRef);
      setLocalAvatarAssetUrl(resolvedUrl);
      setError(null);
    } catch (assetError) {
      setError(assetError instanceof Error ? assetError.message : t('errors.importAvatarFailed'));
    } finally {
      setPending(false);
    }
  }

  async function handleSampleAvatarImport() {
    const response = await fetch(DEFAULT_AVATAR_ASSET_URL);
    if (!response.ok) {
      throw new Error(t('errors.sampleFetchFailed', { status: response.status }));
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
