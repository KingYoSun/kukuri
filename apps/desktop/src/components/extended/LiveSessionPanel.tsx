import type { FormEventHandler } from 'react';
import { useTranslation } from 'react-i18next';

import { Button } from '@/components/ui/button';
import { Card, CardHeader } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Notice } from '@/components/ui/notice';
import { Textarea } from '@/components/ui/textarea';
import { formatLocalizedTime } from '@/i18n/format';

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
  const { t } = useTranslation(['common', 'live']);
  return (
    <Card className='panel-subsection'>
      <CardHeader>
        <h3>{t('live:title')}</h3>
        <small>{t('live:summary', { count: sessions.length })}</small>
      </CardHeader>

      {status === 'loading' ? <Notice>{t('live:loading')}</Notice> : null}
      {status === 'error' && error ? <Notice tone='destructive'>{error}</Notice> : null}

      <form className='composer composer-compact' onSubmit={onSubmit} aria-busy={createPending}>
        <Label>
          <span>{t('live:fields.title')}</span>
          <Input
            value={title}
            onChange={(event) => onTitleChange(event.target.value)}
            placeholder={t('live:fields.placeholders.title')}
            disabled={createPending}
          />
        </Label>
        <Label>
          <span>{t('live:fields.description')}</span>
          <Textarea
            value={description}
            onChange={(event) => onDescriptionChange(event.target.value)}
            placeholder={t('live:fields.placeholders.description')}
            disabled={createPending}
          />
        </Label>
        {status !== 'error' && error ? <p className='error error-inline'>{error}</p> : null}
        <div className='topic-diagnostic topic-diagnostic-secondary'>
          <span>{t('common:labels.audience')}: {audienceLabel}</span>
        </div>
        <Button type='submit' disabled={createPending}>
          {t('live:actions.start')}
        </Button>
      </form>

      {sessions.length === 0 && status === 'ready' ? (
        <p className='empty-state'>{t('live:empty')}</p>
      ) : null}

      <ul className='post-list'>
        {sessions.map(({ session, isOwner, pending }) => (
          <li key={session.session_id}>
            <article className='post-card' aria-busy={pending}>
              <div className='post-meta'>
                <span>{session.title}</span>
                <span>{t(`live:statuses.${session.status}`)}</span>
                <span className='reply-chip'>{session.audience_label}</span>
              </div>
              <div className='post-body'>
                <strong className='post-title'>{session.description || t('common:fallbacks.noDescription')}</strong>
              </div>
              <small>{session.session_id}</small>
              <div className='topic-diagnostic topic-diagnostic-secondary'>
                <span>{t('common:labels.viewers')}: {session.viewer_count}</span>
                <span>{t('common:labels.started')}: {formatLocalizedTime(session.started_at)}</span>
              </div>
              {session.ended_at ? (
                <div className='topic-diagnostic topic-diagnostic-secondary'>
                  <span>{t('common:labels.ended')}: {formatLocalizedTime(session.ended_at)}</span>
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
                    {t('common:actions.leave')}
                  </Button>
                ) : (
                  <Button
                    variant='secondary'
                    type='button'
                    disabled={pending || session.status === 'Ended'}
                    onClick={() => onJoin(session.session_id)}
                  >
                    {t('common:actions.join')}
                  </Button>
                )}
                {isOwner ? (
                  <Button
                    variant='secondary'
                    type='button'
                    disabled={pending || session.status === 'Ended'}
                    onClick={() => onEnd(session.session_id)}
                  >
                    {t('common:actions.end')}
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
