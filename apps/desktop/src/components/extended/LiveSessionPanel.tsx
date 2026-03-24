import type { FormEventHandler } from 'react';

import { Button } from '@/components/ui/button';
import { Card, CardHeader } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Notice } from '@/components/ui/notice';
import { Textarea } from '@/components/ui/textarea';

import { type ExtendedPanelStatus, type LiveSessionListItemView } from './types';

type LiveSessionPanelProps = {
  status: ExtendedPanelStatus;
  error: string | null;
  audienceLabel: string;
  title: string;
  description: string;
  createPending: boolean;
  sessions: LiveSessionListItemView[];
  onTitleChange: (value: string) => void;
  onDescriptionChange: (value: string) => void;
  onSubmit: FormEventHandler<HTMLFormElement>;
  onJoin: (sessionId: string) => void;
  onLeave: (sessionId: string) => void;
  onEnd: (sessionId: string) => void;
};

export function LiveSessionPanel({
  status,
  error,
  audienceLabel,
  title,
  description,
  createPending,
  sessions,
  onTitleChange,
  onDescriptionChange,
  onSubmit,
  onJoin,
  onLeave,
  onEnd,
}: LiveSessionPanelProps) {
  return (
    <Card className='panel-subsection'>
      <CardHeader>
        <h3>Live Sessions</h3>
        <small>{sessions.length} active</small>
      </CardHeader>

      {status === 'loading' ? <Notice>Loading live sessions…</Notice> : null}
      {status === 'error' && error ? <Notice tone='destructive'>{error}</Notice> : null}

      <form className='composer composer-compact' onSubmit={onSubmit} aria-busy={createPending}>
        <Label>
          <span>Live Title</span>
          <Input
            value={title}
            onChange={(event) => onTitleChange(event.target.value)}
            placeholder='Friday stream'
            disabled={createPending}
          />
        </Label>
        <Label>
          <span>Live Description</span>
          <Textarea
            value={description}
            onChange={(event) => onDescriptionChange(event.target.value)}
            placeholder='short session summary'
            disabled={createPending}
          />
        </Label>
        {status !== 'error' && error ? <p className='error error-inline'>{error}</p> : null}
        <div className='topic-diagnostic topic-diagnostic-secondary'>
          <span>Audience: {audienceLabel}</span>
        </div>
        <Button type='submit' disabled={createPending}>
          Start Live
        </Button>
      </form>

      {sessions.length === 0 && status === 'ready' ? (
        <p className='empty-state'>No live sessions</p>
      ) : null}

      <ul className='post-list'>
        {sessions.map(({ session, isOwner, pending }) => (
          <li key={session.session_id}>
            <article className='post-card' aria-busy={pending}>
              <div className='post-meta'>
                <span>{session.title}</span>
                <span>{session.status}</span>
                <span className='reply-chip'>{session.audience_label}</span>
              </div>
              <div className='post-body'>
                <strong className='post-title'>{session.description || 'no description'}</strong>
              </div>
              <small>{session.session_id}</small>
              <div className='topic-diagnostic topic-diagnostic-secondary'>
                <span>viewers: {session.viewer_count}</span>
                <span>started: {new Date(session.started_at).toLocaleTimeString('ja-JP')}</span>
              </div>
              {session.ended_at ? (
                <div className='topic-diagnostic topic-diagnostic-secondary'>
                  <span>ended: {new Date(session.ended_at).toLocaleTimeString('ja-JP')}</span>
                </div>
              ) : null}
              <div className='post-actions'>
                {session.joined_by_me ? (
                  <Button
                    variant='secondary'
                    type='button'
                    disabled={pending}
                    onClick={() => onLeave(session.session_id)}
                  >
                    Leave
                  </Button>
                ) : (
                  <Button
                    variant='secondary'
                    type='button'
                    disabled={pending || session.status === 'Ended'}
                    onClick={() => onJoin(session.session_id)}
                  >
                    Join
                  </Button>
                )}
                {isOwner ? (
                  <Button
                    variant='secondary'
                    type='button'
                    disabled={pending || session.status === 'Ended'}
                    onClick={() => onEnd(session.session_id)}
                  >
                    End
                  </Button>
                ) : null}
              </div>
            </article>
          </li>
        ))}
      </ul>
    </Card>
  );
}
