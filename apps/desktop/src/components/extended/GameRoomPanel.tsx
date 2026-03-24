import type { FormEventHandler } from 'react';

import { Button } from '@/components/ui/button';
import { Card, CardHeader } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Notice } from '@/components/ui/notice';
import { Select } from '@/components/ui/select';
import { Textarea } from '@/components/ui/textarea';
import type { GameRoomStatus, GameRoomView } from '@/lib/api';

import { type ExtendedPanelStatus, type GameDraftView, type GameRoomPendingMap } from './types';

type GameRoomPanelProps = {
  status: ExtendedPanelStatus;
  error: string | null;
  audienceLabel: string;
  title: string;
  description: string;
  participantsInput: string;
  createPending: boolean;
  rooms: GameRoomView[];
  drafts: Record<string, GameDraftView>;
  savingByRoomId: GameRoomPendingMap;
  localAuthorPubkey: string;
  onTitleChange: (value: string) => void;
  onDescriptionChange: (value: string) => void;
  onParticipantsChange: (value: string) => void;
  onSubmit: FormEventHandler<HTMLFormElement>;
  onDraftStatusChange: (roomId: string, status: GameRoomStatus) => void;
  onDraftPhaseChange: (roomId: string, value: string) => void;
  onDraftScoreChange: (roomId: string, participantId: string, value: string) => void;
  onSaveRoom: (roomId: string) => void;
};

export function GameRoomPanel({
  status,
  error,
  audienceLabel,
  title,
  description,
  participantsInput,
  createPending,
  rooms,
  drafts,
  savingByRoomId,
  localAuthorPubkey,
  onTitleChange,
  onDescriptionChange,
  onParticipantsChange,
  onSubmit,
  onDraftStatusChange,
  onDraftPhaseChange,
  onDraftScoreChange,
  onSaveRoom,
}: GameRoomPanelProps) {
  return (
    <Card className='panel-subsection'>
      <CardHeader>
        <h3>Game Rooms</h3>
        <small>{rooms.length} tracked</small>
      </CardHeader>

      {status === 'loading' ? <Notice>Loading game rooms…</Notice> : null}
      {status === 'error' && error ? <Notice tone='destructive'>{error}</Notice> : null}

      <form className='composer composer-compact' onSubmit={onSubmit} aria-busy={createPending}>
        <Label>
          <span>Game Title</span>
          <Input
            value={title}
            onChange={(event) => onTitleChange(event.target.value)}
            placeholder='Top 8 Finals'
            disabled={createPending}
          />
        </Label>
        <Label>
          <span>Game Description</span>
          <Textarea
            value={description}
            onChange={(event) => onDescriptionChange(event.target.value)}
            placeholder='match summary'
            disabled={createPending}
          />
        </Label>
        <Label>
          <span>Participants</span>
          <Input
            value={participantsInput}
            onChange={(event) => onParticipantsChange(event.target.value)}
            placeholder='Alice, Bob'
            disabled={createPending}
          />
        </Label>
        {status !== 'error' && error ? <p className='error error-inline'>{error}</p> : null}
        <div className='topic-diagnostic topic-diagnostic-secondary'>
          <span>Audience: {audienceLabel}</span>
        </div>
        <Button type='submit' disabled={createPending}>
          Create Room
        </Button>
      </form>

      {rooms.length === 0 && status === 'ready' ? <p className='empty-state'>No game rooms</p> : null}

      <ul className='post-list'>
        {rooms.map((room) => {
          const draft = drafts[room.room_id];
          const isOwner = room.host_pubkey === localAuthorPubkey;
          const pending = Boolean(savingByRoomId[room.room_id]);

          return (
            <li key={room.room_id}>
              <article className='post-card' aria-busy={pending}>
                <div className='post-meta'>
                  <span>{room.title}</span>
                  <span>{room.status}</span>
                  <span className='reply-chip'>{room.audience_label}</span>
                </div>
                <div className='post-body'>
                  <strong className='post-title'>{room.description || 'no description'}</strong>
                </div>
                <small>{room.room_id}</small>
                <div className='topic-diagnostic topic-diagnostic-secondary'>
                  <span>phase: {room.phase_label ?? 'none'}</span>
                  <span>updated: {new Date(room.updated_at).toLocaleTimeString('ja-JP')}</span>
                </div>
                <ul className='draft-attachment-list'>
                  {room.scores.map((score) => (
                    <li key={score.participant_id} className='draft-attachment-item score-row'>
                      <div className='draft-attachment-content'>
                        <strong>{score.label}</strong>
                      </div>
                      {isOwner ? (
                        <Input
                          aria-label={`${room.room_id}-${score.label}-score`}
                          value={draft?.scores[score.participant_id] ?? String(score.score)}
                          disabled={pending}
                          onChange={(event) =>
                            onDraftScoreChange(room.room_id, score.participant_id, event.target.value)
                          }
                        />
                      ) : (
                        <span>{score.score}</span>
                      )}
                    </li>
                  ))}
                </ul>
                {isOwner && draft ? (
                  <div className='composer composer-compact'>
                    <Label>
                      <span>Status</span>
                      <Select
                        aria-label={`${room.room_id}-status`}
                        value={draft.status}
                        disabled={pending}
                        onChange={(event) =>
                          onDraftStatusChange(room.room_id, event.target.value as GameRoomStatus)
                        }
                      >
                        <option value='Waiting'>Waiting</option>
                        <option value='Running'>Running</option>
                        <option value='Paused'>Paused</option>
                        <option value='Ended'>Ended</option>
                      </Select>
                    </Label>
                    <Label>
                      <span>Phase</span>
                      <Input
                        aria-label={`${room.room_id}-phase`}
                        value={draft.phaseLabel}
                        disabled={pending}
                        onChange={(event) => onDraftPhaseChange(room.room_id, event.target.value)}
                      />
                    </Label>
                    <Button
                      variant='secondary'
                      type='button'
                      disabled={pending}
                      onClick={() => onSaveRoom(room.room_id)}
                    >
                      Save Room
                    </Button>
                  </div>
                ) : null}
              </article>
            </li>
          );
        })}
      </ul>
    </Card>
  );
}
