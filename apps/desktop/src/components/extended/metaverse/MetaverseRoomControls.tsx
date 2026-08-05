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
import { useTranslation } from 'react-i18next';

import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import type { SupportedLocale } from '@/i18n';
import { formatLocalizedTime } from '@/i18n/format';
import { topicDisplayName } from '@/lib/topicId';
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
  const { t } = useTranslation('metaverse', { lng: locale });
  const avatarStatusLabel = t(`hud.avatarStatuses.${avatarAssetStatus}`);
  return (
    <>
      <div
        className='metaverse-connection-badge'
        data-state={connectionState}
        title={t(`connection.details.${connectionState}`)}
      >
        <ConnectionStateIcon state={connectionState} />
        <span>{t(`connection.states.${connectionState}`)}</span>
      </div>
      <div className='metaverse-hud-toolbar' data-open={hudOpen}>
        <Button
          variant='ghost'
          size='icon'
          className='metaverse-hud-icon-button'
          type='button'
          aria-label={t('hud.leave')}
          onClick={onLeaveRoom}
        >
          <LogOut className='size-4' aria-hidden='true' />
        </Button>
        <Button
          variant='ghost'
          size='icon'
          className='metaverse-hud-icon-button'
          type='button'
          aria-label={t(hudOpen ? 'hud.hide' : 'hud.open')}
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
                <span>{t('hud.debug')}</span>
                <ChevronDown className='size-4' aria-hidden='true' />
              </button>
              {hudDebugOpen ? (
                <div className='metaverse-room-diagnostics'>
                  <span>{t('hud.topic', { topic: topicDisplayName(activeTopic) })}</span>
                  <span>{t('hud.localPeer', { peer: localPeerId })}</span>
                  <span>{t('hud.knownPeers', { count: knownPeerCount })}</span>
                  <span>{t('hud.lastSentSeq', { seq: lastSentSeq })}</span>
                  <span>
                    {t('hud.lastReceived', {
                      time: lastReceivedAt ? formatLocalizedTime(lastReceivedAt, locale) : t('hud.none'),
                    })}
                  </span>
                  <span>{t('hud.remoteAnimation', { value: remoteAnimationSummary || t('hud.none') })}</span>
                  <span>{t('hud.avatarAsset', { status: avatarStatusLabel })}</span>
                  <span>
                    {t('hud.blobResolve', {
                      value: localAvatarAssetRef?.blob_hash ?? t('hud.publicFallback'),
                    })}
                  </span>
                  <span>{t('hud.persistence', { value: room.manifest_blob_hash ?? t('room.pending') })}</span>
                  <span>
                    {t('hud.communityAssist', {
                      value: t(communityAssistAvailable ? 'hud.assistAvailable' : 'hud.assistOptional'),
                    })}
                  </span>
                </div>
              ) : null}
            </section>
            <div className='metaverse-object-controls'>
              <strong>{t('avatar.title')}</strong>
              <div className='metaverse-avatar-asset-controls'>
                <Label>
                  <span className='sr-only'>{t('avatar.fileLabel')}</span>
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
                  {t('avatar.default')}
                </Button>
              </div>
            </div>
            <div className='metaverse-object-controls'>
              <strong>
                <Box className='size-4' aria-hidden='true' />
                {t('object.title')}
              </strong>
              <div className='metaverse-nudge-grid'>
                <Button
                  size='sm'
                  variant='secondary'
                  type='button'
                  onClick={() => onMoveSharedObject([0, 0, -50])}
                >
                  <Move3D className='size-4' aria-hidden='true' />
                  {t('object.forward')}
                </Button>
                <Button size='sm' variant='secondary' type='button' onClick={() => onMoveSharedObject([-50, 0, 0])}>
                  {t('object.left')}
                </Button>
                <Button size='sm' variant='secondary' type='button' onClick={() => onMoveSharedObject([50, 0, 0])}>
                  {t('object.right')}
                </Button>
                <Button size='sm' variant='secondary' type='button' onClick={() => onMoveSharedObject([0, 0, 50])}>
                  {t('object.back')}
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
        <section className='metaverse-room-chat-log' aria-label={t('chat.title')}>
          <div className='metaverse-room-chat-log-header'>
            <span>
              <MessageSquare className='size-4' aria-hidden='true' />
              {t('chat.title')}
            </span>
            <Button
              variant='ghost'
              size='icon'
              className='metaverse-chat-close-button'
              type='button'
              aria-label={t('chat.hide')}
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
                    ? t('chat.you')
                    : message.displayName || message.authorPeerId.slice(0, 12)}
                  <small>{formatLocalizedTime(message.createdAt, locale)}</small>
                </strong>
                <span>{message.body}</span>
              </li>
            ))}
          </ul>
          <form className='metaverse-chat-form' onSubmit={onSendMessage}>
            <Label>
              <span className='sr-only'>{t('chat.messageLabel')}</span>
              <Input
                ref={messageInputRef}
                value={messageDraft}
                placeholder={t('chat.placeholder')}
                onChange={(event) => onMessageDraftChange(event.target.value)}
              />
            </Label>
            <Button size='sm' type='submit'>
              <Send className='size-4' aria-hidden='true' />
              {t('chat.send')}
            </Button>
          </form>
        </section>
      ) : (
        <Button
          variant='secondary'
          size='icon'
          className='metaverse-chat-toggle'
          type='button'
          aria-label={t('chat.open')}
          onClick={onOpenChat}
        >
          <MessageSquare className='size-4' aria-hidden='true' />
        </Button>
      )}
    </>
  );
}
