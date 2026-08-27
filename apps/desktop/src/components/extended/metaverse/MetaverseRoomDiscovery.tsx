import { useState, type FormEvent } from 'react';
import { ChevronDown, Cuboid, Play } from 'lucide-react';
import { useTranslation } from 'react-i18next';

import { AuthorAvatar } from '@/components/core/AuthorAvatar';
import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Notice } from '@/components/ui/notice';
import { Textarea } from '@/components/ui/textarea';
import type { SupportedLocale } from '@/i18n';
import { formatLocalizedTime } from '@/i18n/format';
import type { AuthorSocialView, GameRoomView, Profile, SpatialContextV1 } from '@/lib/api';

export type CreateMetaverseRoomInput = {
  title: string;
  description: string;
  maxPeers: number | null;
};

type MetaverseRoomDiscoveryProps = {
  rooms: GameRoomView[];
  selectedRoomId: string | null;
  joinedRoomIds: ReadonlySet<string>;
  pending: boolean;
  error: string | null;
  locale: SupportedLocale;
  localAuthorPubkey: string;
  localProfile: Profile | null;
  knownAuthorsByPubkey: Record<string, AuthorSocialView>;
  mediaObjectUrls: Record<string, string | null>;
  onCreateRoom: (input: CreateMetaverseRoomInput) => Promise<boolean>;
  onJoinRoom: (roomId: string) => void;
  onMoveRoom?: (roomId: string, targetContext: SpatialContextV1) => Promise<boolean>;
};

export function MetaverseRoomDiscovery({
  rooms,
  selectedRoomId,
  joinedRoomIds,
  pending,
  error,
  locale,
  localAuthorPubkey,
  localProfile,
  knownAuthorsByPubkey,
  mediaObjectUrls,
  onCreateRoom,
  onJoinRoom,
  onMoveRoom,
}: MetaverseRoomDiscoveryProps) {
  const [createOpen, setCreateOpen] = useState(false);
  const [title, setTitle] = useState('');
  const [description, setDescription] = useState('');
  const [maxPeers, setMaxPeers] = useState('8');
  const { t } = useTranslation('metaverse', { lng: locale });
  const [validationError, setValidationError] = useState(false);
  const [movingRoomId, setMovingRoomId] = useState<string | null>(null);
  const [targetTopic, setTargetTopic] = useState('');
  const [targetChannel, setTargetChannel] = useState('');

  function hostAuthor(room: GameRoomView): Profile | AuthorSocialView | null {
    return room.host_pubkey === localAuthorPubkey
      ? localProfile
      : knownAuthorsByPubkey[room.host_pubkey] ?? null;
  }

  function hostLabel(room: GameRoomView) {
    const host = hostAuthor(room);
    return host?.display_name?.trim() || host?.name?.trim() || room.host_pubkey.slice(0, 10);
  }

  function hostPicture(room: GameRoomView) {
    const host = hostAuthor(room);
    const pictureAssetHash = host?.picture_asset?.hash;
    if (pictureAssetHash && typeof mediaObjectUrls[pictureAssetHash] === 'string') {
      return mediaObjectUrls[pictureAssetHash];
    }
    return host?.picture ?? null;
  }

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!title.trim()) {
      setValidationError(true);
      return;
    }
    setValidationError(false);
    const parsedMaxPeers = Number.parseInt(maxPeers, 10);
    const created = await onCreateRoom({
      title: title.trim(),
      description: description.trim(),
      maxPeers: Number.isNaN(parsedMaxPeers) ? null : parsedMaxPeers,
    });
    if (!created) {
      return;
    }
    setTitle('');
    setDescription('');
    setMaxPeers('8');
    setValidationError(false);
  }

  async function handleMoveSubmit(event: FormEvent<HTMLFormElement>, roomId: string) {
    event.preventDefault();
    const topicId = targetTopic.trim();
    if (!topicId || !onMoveRoom) return;
    const channelId = targetChannel.trim();
    const targetContext: SpatialContextV1 = channelId
      ? { kind: 'channel', topic_id: topicId, channel_id: channelId }
      : { kind: 'topic', topic_id: topicId };
    const moved = await onMoveRoom(roomId, targetContext);
    if (moved) {
      setMovingRoomId(null);
      setTargetTopic('');
      setTargetChannel('');
    }
  }

  return (
    <Card className='shell-workspace-card metaverse-discovery-card'>
      <div className='panel-header'>
        <div>
          <h3>{t('title')}</h3>
          <small>{t('rooms.summary', { count: rooms.length })}</small>
        </div>
      </div>
      {validationError || error ? (
        <Notice tone='destructive'>{validationError ? t('create.titleRequired') : error}</Notice>
      ) : null}
      {!rooms.some((room) => room.host_pubkey === localAuthorPubkey) ? (
      <section className='shell-nav-accordion metaverse-create-accordion' data-open={createOpen}>
        <button
          className='shell-nav-accordion-trigger'
          type='button'
          aria-expanded={createOpen}
          onClick={() => setCreateOpen((current) => !current)}
        >
          <Cuboid className='size-4' aria-hidden='true' />
          <span className='shell-nav-accordion-title'>{t('create.action')}</span>
          <ChevronDown className='shell-nav-accordion-icon size-4' aria-hidden='true' />
        </button>
        {createOpen ? (
          <form className='composer composer-compact metaverse-create-form' onSubmit={handleSubmit}>
            <div className='metaverse-create-form-primary'>
              <Label>
                <span>{t('create.titleLabel')}</span>
                <Input value={title} placeholder={t('create.titlePlaceholder')} disabled={pending} onChange={(event) => setTitle(event.target.value)} />
              </Label>
              <Label>
                <span>{t('create.maxPeersLabel')}</span>
                <Input value={maxPeers} disabled={pending} onChange={(event) => setMaxPeers(event.target.value)} />
              </Label>
            </div>
            <Label className='metaverse-create-form-description'>
              <span>{t('create.descriptionLabel')}</span>
              <Textarea value={description} placeholder={t('create.descriptionPlaceholder')} disabled={pending} onChange={(event) => setDescription(event.target.value)} />
            </Label>
            <div className='metaverse-create-form-actions'>
              <Button type='submit' disabled={pending}>
                <Cuboid className='size-4' aria-hidden='true' />
                {t('create.action')}
              </Button>
            </div>
          </form>
        ) : null}
      </section>
      ) : null}
      {rooms.length === 0 ? <p className='empty-state'>{t('rooms.empty')}</p> : null}
      <ul className='metaverse-room-grid'>
        {rooms.map((room) => (
          <li key={room.room_id}>
            <article className={`metaverse-room-card${selectedRoomId === room.room_id ? ' metaverse-room-card-active' : ''}`}>
              <div className='post-meta'>
                <span>{room.title}</span>
                <span>{room.status}</span>
                <span className='reply-chip'>{room.audience_label}</span>
              </div>
              <p>{room.description || t('rooms.noDescription')}</p>
              <div className='metaverse-room-host'>
                <AuthorAvatar label={hostLabel(room)} picture={hostPicture(room)} size='sm' />
                <span>{t('room.host', { host: hostLabel(room) })}</span>
              </div>
              <div className='topic-diagnostic topic-diagnostic-secondary'>
                <span>{t('room.updated', { time: formatLocalizedTime(room.updated_at, locale) })}</span>
                <span>{t(joinedRoomIds.has(room.room_id) ? 'room.joined' : 'room.notJoined')}</span>
              </div>
              <div className='topic-diagnostic topic-diagnostic-secondary'>
                <span>{t('room.manifest', { value: room.manifest_blob_hash ?? t('room.pending') })}</span>
                <span>{t('room.world', { version: room.metaverse?.world_version ?? 1 })}</span>
              </div>
              <Button variant='secondary' type='button' onClick={() => onJoinRoom(room.room_id)}>
                <Play className='size-4' aria-hidden='true' />
                {t('room.join')}
              </Button>
              {onMoveRoom && room.host_pubkey === localAuthorPubkey ? (
                <>
                  <Button
                    variant='secondary'
                    type='button'
                    aria-expanded={movingRoomId === room.room_id}
                    onClick={() => setMovingRoomId((current) => current === room.room_id ? null : room.room_id)}
                  >
                    {t('move.action')}
                  </Button>
                  {movingRoomId === room.room_id ? (
                    <form className='composer composer-compact' onSubmit={(event) => void handleMoveSubmit(event, room.room_id)}>
                      <Label>
                        <span>{t('move.topicLabel')}</span>
                        <Input
                          value={targetTopic}
                          required
                          disabled={pending}
                          placeholder='kukuri:topic:...'
                          onChange={(event) => setTargetTopic(event.target.value)}
                        />
                      </Label>
                      <Label>
                        <span>{t('move.channelLabel')}</span>
                        <Input
                          value={targetChannel}
                          disabled={pending}
                          placeholder={t('move.channelPlaceholder')}
                          onChange={(event) => setTargetChannel(event.target.value)}
                        />
                      </Label>
                      <Button type='submit' disabled={pending || !targetTopic.trim()}>
                        {pending ? t('move.moving') : t('move.confirm')}
                      </Button>
                    </form>
                  ) : null}
                </>
              ) : null}
            </article>
          </li>
        ))}
      </ul>
    </Card>
  );
}
