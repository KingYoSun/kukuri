import { type ReactNode, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Flag } from 'lucide-react';

import { Card, CardHeader } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import type {
  CommunityNodeManifestFetch,
  SubmitCommunityNodeReportRequest,
  SubmitCommunityNodeReportResult,
} from '@/lib/api';
import { contentProvenanceFromView } from '@/lib/api/provenance';
import { planReportRouting } from '@/lib/api/reportRouting';

import { AuthorAvatar } from './AuthorAvatar';
import { RelationshipBadge } from './RelationshipBadge';
import { ReportRoutingDialog, type ReportSubmitInput } from './ReportRoutingDialog';
import { useReportManifests } from './useReportManifests';
import { type AuthorDetailView } from './types';

type AuthorDetailCardProps = {
  view: AuthorDetailView;
  localAuthorPubkey: string;
  onToggleRelationship: (authorPubkey: string, following: boolean) => void;
  onToggleMute: (authorPubkey: string, muted: boolean) => void;
  onToggleBlock?: (authorPubkey: string, blocking: boolean) => void;
  onOpenDirectMessage?: (authorPubkey: string) => void;
  communityNodeAdvisory?: ReactNode;
  onSubmitReport?: (
    request: SubmitCommunityNodeReportRequest
  ) => Promise<SubmitCommunityNodeReportResult>;
  onCopyReportContact?: (value: string) => void;
  onFetchReportManifest?: (baseUrl: string) => Promise<CommunityNodeManifestFetch>;
};

export function AuthorDetailCard({
  view,
  localAuthorPubkey,
  onToggleRelationship,
  onToggleMute,
  onToggleBlock,
  onOpenDirectMessage,
  communityNodeAdvisory,
  onSubmitReport,
  onCopyReportContact,
  onFetchReportManifest,
}: AuthorDetailCardProps) {
  const { t } = useTranslation(['common']);
  const author = view.author;
  const relationshipLabel = view.summary?.label ?? null;
  const showFollowAction = author?.author_pubkey !== localAuthorPubkey;
  const showMessageAction = Boolean(
    author &&
      author.author_pubkey !== localAuthorPubkey &&
      view.canMessage &&
      onOpenDirectMessage
  );
  const showMuteAction = Boolean(author && author.author_pubkey !== localAuthorPubkey);
  const [reportOpen, setReportOpen] = useState(false);
  const provenance = useMemo(
    () => contentProvenanceFromView(author?.provenance),
    [author?.provenance]
  );
  // 通報画面を開いた時に取得成功した最新 manifest だけを候補源にする(#696)。
  const {
    manifests: reportManifests,
    resolving: reportResolving,
    resolveError: reportResolveError,
  } = useReportManifests({
    open: reportOpen,
    provenance,
    fetchManifest: onFetchReportManifest,
  });
  const reportPlan = useMemo(
    () => planReportRouting(provenance, reportManifests),
    [provenance, reportManifests]
  );

  const submitReport = async (input: ReportSubmitInput) => {
    if (!author || !onSubmitReport) throw new Error('report submission is not available');
    return onSubmitReport({
      node_base_url: input.candidate.target.nodeBaseUrl,
      report_endpoint: input.candidate.target.reportEndpoint ?? '',
      subject_kind: 'profile',
      subject_id: author.author_pubkey,
      capability: input.candidate.target.capability,
      reason: input.reason,
      details: input.details.trim() || null,
      reporter_contact: input.reporterContact.trim() || null,
    });
  };

  return (
    <Card className='author-detail'>
      {author ? (
        <>
          <CardHeader className='author-detail-toolbar'>
            <div className='author-detail-summary'>
              <div className='author-detail-hero'>
                <AuthorAvatar
                  label={view.displayLabel}
                  picture={view.pictureSrc ?? author.picture ?? null}
                  size='sm'
                  testId='author-detail-avatar'
                />
                <div className='author-detail-identity'>
                  <div className='author-detail-heading'>
                    <strong className='author-detail-name author-detail-break'>{view.displayLabel}</strong>
                    {relationshipLabel ? (
                      <RelationshipBadge
                        label={relationshipLabel}
                        className='author-detail-relationship'
                      />
                    ) : null}
                  </div>
                </div>
              </div>
              <div className='author-detail-copy-stack'>
                <p className='author-detail-copy author-detail-break'>
                  {author.about?.trim() || t('fallbacks.noBio')}
                </p>
                <small className='author-detail-monotext'>{author.author_pubkey}</small>
              </div>
            </div>
          </CardHeader>

          {view.summary && view.summary.viaPubkeys.length > 0 ? (
            <div className='topic-diagnostic topic-diagnostic-secondary'>
              <span>{t('relationships.via')}</span>
              <p className='author-detail-break'>{view.summary.viaPubkeys.join(', ')}</p>
            </div>
          ) : null}

          {showFollowAction || showMuteAction || showMessageAction ? (
            <div className='author-detail-actions'>
              <div className='author-detail-action-buttons'>
                {showMessageAction ? (
                  <button
                    className='button button-secondary'
                    type='button'
                    onClick={() => onOpenDirectMessage?.(author.author_pubkey)}
                  >
                    {t('actions.message', { defaultValue: 'Message' })}
                  </button>
                ) : null}
                {showFollowAction ? (
                  <button
                    className='button button-secondary'
                    type='button'
                    onClick={() => onToggleRelationship(author.author_pubkey, author.following)}
                  >
                    {view.summary?.followActionLabel === 'Unfollow'
                      ? t('actions.unfollow')
                      : t('actions.follow')}
                  </button>
                ) : null}
                {showMuteAction ? (
                  <button
                    className='button button-secondary'
                    type='button'
                    onClick={() => onToggleMute(author.author_pubkey, author.muted)}
                  >
                    {view.summary?.muteActionLabel === 'Unmute'
                      ? t('actions.unmute', { defaultValue: 'Unmute' })
                      : t('actions.mute', { defaultValue: 'Mute' })}
                  </button>
                ) : null}
                {showMuteAction && onToggleBlock ? (
                  <button
                    className='button button-secondary'
                    type='button'
                    onClick={() => onToggleBlock(author.author_pubkey, author.blocking)}
                  >
                    {t(author.blocking ? 'actions.unblock' : 'actions.block')}
                  </button>
                ) : null}
                {author && onSubmitReport ? (
                  <Button
                    variant='secondary'
                    size='icon'
                    type='button'
                    aria-label={t('report.actionLabel', { ns: 'shell' })}
                    onClick={() => setReportOpen(true)}
                  >
                    <Flag className='size-4' aria-hidden='true' />
                  </Button>
                ) : null}
              </div>
            </div>
          ) : null}
          {communityNodeAdvisory}
          {author && onSubmitReport ? (
            <ReportRoutingDialog
              open={reportOpen}
              onOpenChange={setReportOpen}
              subject={{ kind: 'profile', id: author.author_pubkey, label: view.displayLabel }}
              plan={reportPlan}
              onSubmit={submitReport}
              onCopyContact={onCopyReportContact}
              resolving={reportResolving}
              resolveError={reportResolveError}
              localActions={
                showMuteAction ? (
                  <Button
                    variant='secondary'
                    type='button'
                    onClick={() => onToggleMute(author.author_pubkey, author.muted)}
                  >
                    {t(author.muted ? 'actions.unmute' : 'actions.mute')}
                  </Button>
                ) : undefined
              }
            />
          ) : null}
        </>
      ) : (
        <p className='empty'>{t('fallbacks.selectAuthor')}</p>
      )}

      {view.authorError ? <p className='error error-inline'>{view.authorError}</p> : null}
    </Card>
  );
}
