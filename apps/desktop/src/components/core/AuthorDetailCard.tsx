import { type ReactNode, useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Flag } from 'lucide-react';

import { Card, CardHeader } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import type {
  CommunityNodeManifest,
  CommunityNodeManifestFetch,
  SubmitCommunityNodeReportRequest,
  SubmitCommunityNodeReportResult,
} from '@/lib/api';
import { contentProvenanceFromView } from '@/lib/api/provenance';
import { planReportRouting } from '@/lib/api/reportRouting';

import { AuthorAvatar } from './AuthorAvatar';
import { RelationshipBadge } from './RelationshipBadge';
import { ReportRoutingDialog, type ReportSubmitInput } from './ReportRoutingDialog';
import { type AuthorDetailView } from './types';

const EMPTY_COMMUNITY_NODE_MANIFESTS: Record<string, CommunityNodeManifest> = {};

type AuthorDetailCardProps = {
  view: AuthorDetailView;
  localAuthorPubkey: string;
  onToggleRelationship: (authorPubkey: string, following: boolean) => void;
  onToggleMute: (authorPubkey: string, muted: boolean) => void;
  onOpenDirectMessage?: (authorPubkey: string) => void;
  communityNodeAdvisory?: ReactNode;
  communityNodeManifests?: Record<string, CommunityNodeManifest>;
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
  onOpenDirectMessage,
  communityNodeAdvisory,
  communityNodeManifests = EMPTY_COMMUNITY_NODE_MANIFESTS,
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
  const [reportManifests, setReportManifests] = useState(communityNodeManifests);
  const fetchedReportManifestUrls = useRef(new Set<string>());
  const [reportResolving, setReportResolving] = useState(false);
  const [reportResolveError, setReportResolveError] = useState<string | null>(null);
  const provenance = useMemo(
    () => contentProvenanceFromView(author?.provenance),
    [author?.provenance]
  );
  const reportPlan = useMemo(
    () => planReportRouting(provenance, reportManifests),
    [provenance, reportManifests]
  );

  useEffect(() => setReportManifests(communityNodeManifests), [communityNodeManifests]);
  useEffect(() => {
    if (!reportOpen) {
      fetchedReportManifestUrls.current.clear();
      setReportResolveError(null);
    }
  }, [reportOpen]);
  useEffect(() => {
    if (!reportOpen || !onFetchReportManifest || !provenance) return;
    const baseUrls = [...new Set(provenance.observedVia.map((item) => item.nodeBaseUrl))]
      .filter((baseUrl) => !fetchedReportManifestUrls.current.has(baseUrl));
    if (baseUrls.length === 0) return;
    baseUrls.forEach((baseUrl) => fetchedReportManifestUrls.current.add(baseUrl));
    let active = true;
    setReportResolving(true);
    setReportResolveError(null);
    Promise.all(
      baseUrls.map(async (baseUrl) => ({
        baseUrl,
        response: await onFetchReportManifest(baseUrl),
      }))
    )
      .then((responses) => {
        if (!active) return;
        setReportManifests((current) => {
          let next = current;
          for (const { baseUrl, response } of responses) {
            if (response.status === 'ok' && response.manifest) {
              if (next === current) next = { ...current };
              next[baseUrl] = response.manifest;
            }
          }
          return next;
        });
        if (responses.some(({ response }) => response.status !== 'ok' || !response.manifest)) {
          setReportResolveError('community node manifest is unavailable');
        }
      })
      .catch((cause) => {
        if (active) setReportResolveError(cause instanceof Error ? cause.message : String(cause));
      })
      .finally(() => {
        if (active) setReportResolving(false);
      });
    return () => {
      active = false;
    };
  }, [onFetchReportManifest, provenance, reportOpen]);

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
