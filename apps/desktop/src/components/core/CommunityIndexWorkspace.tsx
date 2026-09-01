import { type FormEvent, useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { TFunction } from 'i18next';
import { Search } from 'lucide-react';
import { useTranslation } from 'react-i18next';

import type {
  AuthorSocialView,
  BookmarkedCustomReactionView,
  CommunityIndexPostResolveInput,
  CommunityIndexResolvedPostView,
  CommunityNodeIndexQueryRequest,
  CustomReactionAssetView,
  DesktopApi,
  IndexEntryView,
  PostView,
  Profile,
  ReactionKeyInput,
  RecentReactionView,
  TimelineScope,
} from '@/lib/api';
import { InvokeError } from '@/lib/api/invoke/error';
import type { InternalSmartReference } from '@/lib/internalLinks';
import { copyTextToClipboard } from '@/lib/utils';

import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Notice } from '@/components/ui/notice';
import { CommunityNodeConsentDialog } from '@/components/settings/CommunityNodeConsentDialog';
import { communityNodeConsentView } from '@/shell/presentation';
import type { CommunityNodePoliciesEntry } from '@/shell/store';
import { communityIndexPostCardView } from './communityIndexPostCardView';
import { PostCard } from './PostCard';

type IndexOperation = 'search' | 'discovery' | 'recommendations';
const EMPTY_KNOWN_AUTHORS: Record<string, AuthorSocialView> = {};

type CommunityIndexWorkspaceProps = {
  api: DesktopApi;
  mode: 'topic' | 'explore';
  activeTopic: string;
  activeTimelineScope: TimelineScope;
  eligibleNodeBaseUrls: readonly string[];
  // #857: 設定済みだがローカル同意が成立していない(未同意・撤回・再同意待ち)node。
  // Node 機能の利用直前に同意モーダルを提示するために使う。
  consentPendingNodeBaseUrls?: readonly string[];
  selectedNodeBaseUrl: string | null;
  onOpenCommunityNodeSettings: () => void;
  knownAuthorsByPubkey?: Record<string, AuthorSocialView>;
  mediaObjectUrls?: Record<string, string | null>;
  adultContentEnabled?: boolean;
  onOpenAuthor: (pubkey: string) => void;
  onOpenThread?: (threadId: string) => void;
  onOpenThreadInTopic?: (threadId: string, topicId: string) => void;
  onReply?: (post: PostView) => void;
  onRepost?: (post: PostView) => void;
  onQuoteRepost?: (post: PostView) => void;
  localAuthorPubkey?: string;
  localProfile?: Profile | null;
  ownedReactionAssets?: CustomReactionAssetView[];
  bookmarkedReactionAssets?: BookmarkedCustomReactionView[];
  recentReactions?: RecentReactionView[];
  onToggleReaction?: (post: PostView, reactionKey: ReactionKeyInput) => void;
  onBookmarkCustomReaction?: (asset: CustomReactionAssetView) => void;
  onReactionPickerOpen?: () => void;
  showBookmarkAction?: boolean;
  bookmarkedPostIds?: Set<string>;
  onToggleBookmark?: (post: PostView) => void;
  onWithdraw?: (post: PostView) => void;
  onActivateReference?: (reference: InternalSmartReference) => void;
  onCopyPostLink?: (link: string) => void;
};

type IndexRequestContext = {
  key: string;
  nodeBaseUrl: string;
  operation: IndexOperation;
  scopeKind: CommunityNodeIndexQueryRequest['scope_kind'];
  scopeId: CommunityNodeIndexQueryRequest['scope_id'];
  topicId: CommunityNodeIndexQueryRequest['topic_id'];
};

type IndexResultState = {
  context: IndexRequestContext;
  entries: IndexEntryView[];
};

type ResolvedPostState = {
  contextKey: string;
  entriesByKey: Record<string, CommunityIndexResolvedPostView>;
};

type ResolvedAuthorState = {
  contextKey: string;
  entriesByPubkey: Record<
    string,
    { status: 'loading' | 'resolved' | 'failed'; author: AuthorSocialView | null }
  >;
};

function indexEntryKey(entry: IndexEntryView): string {
  return `${entry.scope_kind}:${entry.scope_id}:${entry.object_id}`;
}

function resolveInputForEntry(
  entry: IndexEntryView,
  context: IndexRequestContext
): CommunityIndexPostResolveInput | null {
  const topic =
    entry.scope_kind === 'public_topic' ? entry.scope_id : context.topicId?.trim() || null;
  if (!topic) return null;
  return {
    key: indexEntryKey(entry),
    topic,
    object_id: entry.object_id,
    author_pubkey: entry.author_pubkey,
    channel_ref:
      entry.scope_kind === 'public_topic'
        ? { kind: 'public' }
        : { kind: 'private_channel', channel_id: entry.scope_id },
  };
}

function localAuthorView(profile: Profile, authorPubkey: string): AuthorSocialView {
  return {
    author_pubkey: authorPubkey,
    name: profile.name ?? null,
    display_name: profile.display_name ?? null,
    about: profile.about ?? null,
    picture: profile.picture ?? null,
    picture_asset: profile.picture_asset ?? null,
    updated_at: profile.updated_at,
    following: false,
    followed_by: false,
    mutual: false,
    friend_of_friend: false,
    friend_of_friend_via_pubkeys: [],
    provenance: null,
    muted: false,
    blocking: false,
    blocked_by: false,
  };
}

function operationMethod(api: DesktopApi, operation: IndexOperation) {
  if (operation === 'search') return api.searchCommunityNodeIndex.bind(api);
  if (operation === 'discovery') return api.discoverCommunityNodeIndex.bind(api);
  return api.recommendCommunityNodeIndex.bind(api);
}

function communityIndexErrorMessage(error: unknown, t: TFunction): string {
  if (!(error instanceof InvokeError)) {
    return error instanceof Error ? error.message : String(error);
  }
  if (error.code === 'AUTH_REQUIRED' || error.status === 401) {
    return t('shell:communityIndex.errors.authRequired');
  }
  if (error.code === 'CONSENT_REQUIRED' || error.status === 403) {
    return t('shell:communityIndex.errors.consentRequired');
  }
  if (error.code === 'INDEX_QUERY_NOT_CONFIGURED') {
    return t('shell:communityIndex.errors.notConfigured');
  }
  if (error.code === 'INDEX_QUERY_NOT_ACTIVATED') {
    return t('shell:communityIndex.errors.notActivated');
  }
  if (error.status === 429 || error.code === 'RATE_LIMITED') {
    return error.retryAfterSeconds
      ? t('shell:communityIndex.errors.rateLimitedWithRetry', {
          seconds: error.retryAfterSeconds,
        })
      : t('shell:communityIndex.errors.rateLimited');
  }
  return error.message;
}

function indexContext(
  mode: CommunityIndexWorkspaceProps['mode'],
  operation: IndexOperation,
  selectedNodeBaseUrl: string | null,
  activeTopic: string,
  activeTimelineScope: TimelineScope
): IndexRequestContext | null {
  if (!selectedNodeBaseUrl) return null;
  const scopeKind =
    mode !== 'topic'
      ? null
      : activeTimelineScope.kind === 'channel'
        ? 'private_channel'
        : 'public_topic';
  const scopeId =
    mode !== 'topic'
      ? null
      : activeTimelineScope.kind === 'channel'
        ? activeTimelineScope.channel_id
        : activeTopic;
  return {
    key: [
      selectedNodeBaseUrl,
      operation,
      mode,
      activeTimelineScope.kind,
      scopeKind ?? '',
      scopeId ?? '',
    ].join('\u0000'),
    nodeBaseUrl: selectedNodeBaseUrl,
    operation,
    scopeKind,
    scopeId,
    // 非公開チャンネル範囲では所属証明(channel secret)を引くために topic が要る(#711)。
    topicId: scopeKind === 'private_channel' ? activeTopic : null,
  };
}

export function CommunityIndexWorkspace({
  api,
  mode,
  activeTopic,
  activeTimelineScope,
  eligibleNodeBaseUrls,
  consentPendingNodeBaseUrls = [],
  selectedNodeBaseUrl,
  onOpenCommunityNodeSettings,
  knownAuthorsByPubkey = EMPTY_KNOWN_AUTHORS,
  mediaObjectUrls = {},
  adultContentEnabled = false,
  onOpenAuthor,
  onOpenThread,
  onOpenThreadInTopic,
  onReply,
  onRepost,
  onQuoteRepost,
  localAuthorPubkey,
  localProfile,
  ownedReactionAssets = [],
  bookmarkedReactionAssets = [],
  recentReactions = [],
  onToggleReaction,
  onBookmarkCustomReaction,
  onReactionPickerOpen,
  showBookmarkAction = false,
  bookmarkedPostIds,
  onToggleBookmark,
  onWithdraw,
  onActivateReference,
  onCopyPostLink,
}: CommunityIndexWorkspaceProps) {
  const { t, i18n } = useTranslation(['shell', 'common']);
  const [operation, setOperation] = useState<IndexOperation>('search');
  // #857: Node 機能の利用直前に提示する同意モーダルの状態。提示内容は認証不要の
  // 公開 policy カタログから取得し、同意成立後にのみ認証・接続が始まる。
  const [consentGateBaseUrl, setConsentGateBaseUrl] = useState<string | null>(null);
  const [consentGateEntry, setConsentGateEntry] = useState<CommunityNodePoliciesEntry | null>(null);
  const [consentGateBusy, setConsentGateBusy] = useState(false);
  const [query, setQuery] = useState('');
  const [resultState, setResultState] = useState<IndexResultState | null>(null);
  const [status, setStatus] = useState<'idle' | 'loading' | 'success' | 'error'>('idle');
  const [error, setError] = useState<string | null>(null);
  const [resolvedPostState, setResolvedPostState] = useState<ResolvedPostState | null>(null);
  const [resolvedAuthorState, setResolvedAuthorState] = useState<ResolvedAuthorState | null>(null);
  const requestSequence = useRef(0);
  const detailSequence = useRef(0);
  const authorSequence = useRef(0);

  const effectiveOperation: IndexOperation = mode === 'topic' ? 'search' : operation;
  const isAllJoined = mode === 'topic' && activeTimelineScope.kind === 'all_joined';
  // 選択値が適格一覧(認証・同意・通信・提供中能力)に含まれない間は、再調整が追いつくまで
  // 古いノードへ要求を送らない(#698)。文脈も無効化するので古い結果・通報対象は失効する。
  const activeNodeBaseUrl =
    selectedNodeBaseUrl !== null && eligibleNodeBaseUrls.includes(selectedNodeBaseUrl)
      ? selectedNodeBaseUrl
      : null;
  const disabled = activeNodeBaseUrl === null || isAllJoined;
  const currentContext = useMemo(
    () => indexContext(mode, effectiveOperation, activeNodeBaseUrl, activeTopic, activeTimelineScope),
    [activeNodeBaseUrl, activeTimelineScope, activeTopic, effectiveOperation, mode]
  );
  const currentContextKey = currentContext?.key ?? null;
  const currentContextKeyRef = useRef(currentContextKey);
  const visibleResult =
    resultState && resultState.context.key === currentContextKey ? resultState : null;
  const resolvedPostsByKey = useMemo(
    () =>
      visibleResult && resolvedPostState?.contextKey === visibleResult.context.key
        ? resolvedPostState.entriesByKey
        : {},
    [resolvedPostState, visibleResult]
  );
  const resolvedAuthorsByPubkey = useMemo(
    () =>
      visibleResult && resolvedAuthorState?.contextKey === visibleResult.context.key
        ? resolvedAuthorState.entriesByPubkey
        : {},
    [resolvedAuthorState, visibleResult]
  );
  const visiblePostCards = useMemo(
    () =>
      visibleResult?.entries.map((entry) => {
        const key = indexEntryKey(entry);
        const localAuthor =
          entry.author_pubkey === localAuthorPubkey && localProfile
            ? localAuthorView(localProfile, entry.author_pubkey)
            : null;
        const authorResolution = resolvedAuthorsByPubkey[entry.author_pubkey];
        const knownAuthor =
          localAuthor ??
          knownAuthorsByPubkey[entry.author_pubkey] ??
          authorResolution?.author ??
          null;
        const resolvedEntry = resolvedPostsByKey[key] ?? null;
        return {
          key,
          resolvedEntry,
          view: communityIndexPostCardView(entry, {
            nodeBaseUrl: visibleResult.context.nodeBaseUrl,
            operation: visibleResult.context.operation,
            topicId: visibleResult.context.topicId ?? null,
            knownAuthor,
            authorStatus: knownAuthor ? 'resolved' : authorResolution?.status ?? 'loading',
            resolvedEntry,
            mediaObjectUrls,
            adultContentEnabled,
          }),
        };
      }) ?? [],
    [
      adultContentEnabled,
      knownAuthorsByPubkey,
      localAuthorPubkey,
      localProfile,
      mediaObjectUrls,
      resolvedAuthorsByPubkey,
      resolvedPostsByKey,
      visibleResult,
    ]
  );

  const invalidateResults = useCallback(() => {
    requestSequence.current += 1;
    detailSequence.current += 1;
    authorSequence.current += 1;
    setStatus('idle');
    setResultState(null);
    setResolvedPostState(null);
    setResolvedAuthorState(null);
    setError(null);
  }, []);

  useEffect(() => {
    currentContextKeyRef.current = currentContextKey;
    invalidateResults();
  }, [currentContextKey, invalidateResults]);

  useEffect(() => {
    if (!visibleResult) {
      setResolvedPostState(null);
      return;
    }
    const contextKey = visibleResult.context.key;
    const sequence = ++detailSequence.current;
    const entries = visibleResult.entries.flatMap((entry) => {
      const input = resolveInputForEntry(entry, visibleResult.context);
      return input ? [input] : [];
    });
    setResolvedPostState({ contextKey, entriesByKey: {} });
    if (entries.length === 0 || typeof api.resolveCommunityIndexPosts !== 'function') return;

    void api
      .resolveCommunityIndexPosts(entries)
      .then((response) => {
        if (
          sequence !== detailSequence.current ||
          contextKey !== currentContextKeyRef.current
        ) {
          return;
        }
        setResolvedPostState({
          contextKey,
          entriesByKey: Object.fromEntries(response.entries.map((entry) => [entry.key, entry])),
        });
      })
      .catch(() => {
        if (
          sequence === detailSequence.current &&
          contextKey === currentContextKeyRef.current
        ) {
          setResolvedPostState({ contextKey, entriesByKey: {} });
        }
      });
  }, [api, visibleResult]);

  useEffect(() => {
    if (!visibleResult) {
      setResolvedAuthorState(null);
      return;
    }
    const contextKey = visibleResult.context.key;
    const sequence = ++authorSequence.current;
    const missingPubkeys = Array.from(
      new Set(
        visibleResult.entries
          .map((entry) => entry.author_pubkey)
          .filter(
            (pubkey) =>
              !(pubkey === localAuthorPubkey && localProfile) &&
              !knownAuthorsByPubkey[pubkey]
          )
      )
    );
    setResolvedAuthorState({
      contextKey,
      entriesByPubkey: Object.fromEntries(
        missingPubkeys.map((pubkey) => [pubkey, { status: 'loading', author: null }])
      ),
    });
    if (missingPubkeys.length === 0) return;

    void (async () => {
      const entriesByPubkey: ResolvedAuthorState['entriesByPubkey'] = {};
      for (let offset = 0; offset < missingPubkeys.length; offset += 4) {
        const chunk = missingPubkeys.slice(offset, offset + 4);
        const chunkResults = await Promise.all(
          chunk.map(async (pubkey) => {
            try {
              return {
                pubkey,
                status: 'resolved' as const,
                author: await api.getAuthorSocialView(pubkey),
              };
            } catch {
              return { pubkey, status: 'failed' as const, author: null };
            }
          })
        );
        for (const result of chunkResults) {
          entriesByPubkey[result.pubkey] = {
            status: result.status,
            author: result.author,
          };
        }
      }
      if (
        sequence !== authorSequence.current ||
        contextKey !== currentContextKeyRef.current
      ) {
        return;
      }
      setResolvedAuthorState({ contextKey, entriesByPubkey });
    })();
  }, [api, knownAuthorsByPubkey, localAuthorPubkey, localProfile, visibleResult]);

  const refreshResolvedEntry = useCallback(
    async (key: string) => {
      if (!visibleResult || typeof api.resolveCommunityIndexPosts !== 'function') return;
      const entry = visibleResult.entries.find((candidate) => indexEntryKey(candidate) === key);
      if (!entry) return;
      const input = resolveInputForEntry(entry, visibleResult.context);
      if (!input) return;
      const contextKey = visibleResult.context.key;
      try {
        const response = await api.resolveCommunityIndexPosts([input]);
        if (contextKey !== currentContextKeyRef.current) return;
        const resolvedEntry = response.entries.find((candidate) => candidate.key === key);
        if (!resolvedEntry) return;
        setResolvedPostState((current) =>
          current?.contextKey === contextKey
            ? {
                contextKey,
                entriesByKey: { ...current.entriesByKey, [key]: resolvedEntry },
              }
            : current
        );
      } catch {
        // 操作側のエラー表示を維持し、直前の有効な解決結果は消さない。
      }
    },
    [api, visibleResult]
  );

  async function openConsentGate(baseUrl: string) {
    setConsentGateBaseUrl(baseUrl);
    setConsentGateEntry({ status: 'loading' });
    try {
      const catalog = await api.fetchCommunityNodePolicies(baseUrl);
      setConsentGateEntry({ status: 'ok', policies: catalog.policies });
    } catch (fetchError) {
      // 取得失敗(オフライン等)はモーダル内で再試行できる。
      setConsentGateEntry({
        status: 'error',
        error: fetchError instanceof Error ? fetchError.message : String(fetchError),
      });
    }
  }

  function closeConsentGate() {
    setConsentGateBaseUrl(null);
    setConsentGateEntry(null);
  }

  async function acceptConsentGate() {
    if (consentGateBaseUrl === null || consentGateEntry?.status !== 'ok') {
      return;
    }
    setConsentGateBusy(true);
    try {
      await api.acceptCommunityNodeConsents(
        consentGateBaseUrl,
        consentGateEntry.policies.map((policy) => ({
          policy_slug: policy.policy_slug,
          policy_version: policy.policy_version,
        })),
        i18n.resolvedLanguage ?? i18n.language
      );
      closeConsentGate();
    } catch (acceptError) {
      setError(communityIndexErrorMessage(acceptError, t));
    } finally {
      setConsentGateBusy(false);
    }
  }

  async function runQuery(event?: FormEvent) {
    event?.preventDefault();
    if (disabled || !currentContext) return;
    if (effectiveOperation === 'search' && !query.trim()) {
      setStatus('error');
      setError(t('shell:communityIndex.queryRequired'));
      return;
    }
    const request: CommunityNodeIndexQueryRequest = {
      base_url: currentContext.nodeBaseUrl,
      query: currentContext.operation === 'search' ? query.trim() : null,
      scope_kind: currentContext.scopeKind,
      scope_id: currentContext.scopeId,
      topic_id: currentContext.topicId,
      limit: 50,
    };
    const requestContext = currentContext;
    const sequence = ++requestSequence.current;
    setStatus('loading');
    setError(null);
    try {
      const response = await operationMethod(api, requestContext.operation)(request);
      if (
        sequence !== requestSequence.current ||
        requestContext.key !== currentContextKeyRef.current
      ) {
        return;
      }
      setResultState({ context: requestContext, entries: response.entries });
      setStatus('success');
    } catch (cause) {
      if (
        sequence !== requestSequence.current ||
        requestContext.key !== currentContextKeyRef.current
      ) {
        return;
      }
      setResultState(null);
      setError(communityIndexErrorMessage(cause, t));
      setStatus('error');
    }
  }

  return (
    <Card className='shell-workspace-card space-y-4' data-testid={`community-index-${mode}`}>
      <div className='flex flex-wrap items-start justify-between gap-3'>
        <div className='space-y-1'>
          <h3 className='text-lg font-semibold'>{t('shell:communityIndex.title')}</h3>
          <p className='text-sm text-[var(--muted-foreground)]'>
            {mode === 'topic'
              ? t('shell:communityIndex.topicSummary')
              : t('shell:communityIndex.exploreSummary')}
          </p>
        </div>
      </div>

      {mode === 'explore' ? (
        <div
          className='shell-workspace-tabs shell-community-index-tabs'
          role='tablist'
          aria-label={t('shell:communityIndex.surfaces')}
        >
          {(['search', 'discovery', 'recommendations'] as const).map((value) => (
            <button
              key={value}
              className={`shell-tab${operation === value ? ' shell-tab-active' : ''}`}
              type='button'
              role='tab'
              aria-selected={operation === value}
              onClick={() => {
                invalidateResults();
                setOperation(value);
              }}
            >
              {t(`shell:communityIndex.operations.${value}`)}
            </button>
          ))}
        </div>
      ) : null}

      {consentPendingNodeBaseUrls.length > 0 ? (
        <Notice tone='warning'>
          <div className='flex flex-wrap items-center justify-between gap-3'>
            <span>{t('shell:communityIndex.consentRequiredNotice')}</span>
            <div className='flex flex-wrap gap-2'>
              {consentPendingNodeBaseUrls.map((baseUrl) => (
                <Button
                  key={baseUrl}
                  variant='secondary'
                  type='button'
                  onClick={() => void openConsentGate(baseUrl)}
                >
                  {t('shell:communityIndex.reviewPolicies', { baseUrl })}
                </Button>
              ))}
            </div>
          </div>
        </Notice>
      ) : null}

      {eligibleNodeBaseUrls.length === 0 ? (
        consentPendingNodeBaseUrls.length > 0 ? null : (
        <Notice tone='warning'>
          <div className='flex flex-wrap items-center justify-between gap-3'>
            <span>{t('shell:communityIndex.noEligibleNode')}</span>
            <Button variant='secondary' type='button' onClick={onOpenCommunityNodeSettings}>
              {t('shell:workspace.communityNodeUnavailableAction')}
            </Button>
          </div>
        </Notice>
        )
      ) : activeNodeBaseUrl === null ? (
        <Notice tone='warning'>
          <div className='flex flex-wrap items-center justify-between gap-3'>
            <span>{t('shell:communityIndex.selectedNodeUnavailable')}</span>
            <Button variant='secondary' type='button' onClick={onOpenCommunityNodeSettings}>
              {t('shell:workspace.communityNodeUnavailableAction')}
            </Button>
          </div>
        </Notice>
      ) : isAllJoined ? (
        <Notice tone='warning'>{t('shell:communityIndex.allJoinedDisabled')}</Notice>
      ) : (
        <form className='flex flex-col gap-3 sm:flex-row' onSubmit={(event) => void runQuery(event)}>
          {effectiveOperation === 'search' ? (
            <Input
              aria-label={t('shell:communityIndex.queryLabel')}
              value={query}
              onChange={(event) => setQuery(event.currentTarget.value)}
              placeholder={t('shell:communityIndex.queryPlaceholder')}
            />
          ) : (
            <p className='flex-1 self-center text-sm text-[var(--muted-foreground)]'>
              {t(`shell:communityIndex.operationHints.${effectiveOperation}`)}
            </p>
          )}
          <Button type='submit' disabled={status === 'loading'}>
            <Search className='size-4' aria-hidden='true' />
            {status === 'loading'
              ? t('shell:communityIndex.loading')
              : t('shell:communityIndex.run')}
          </Button>
        </form>
      )}

      {error ? <Notice tone='destructive'>{error}</Notice> : null}
      {status === 'success' && visibleResult && visibleResult.entries.length === 0 ? (
        <p className='empty-state'>{t('shell:communityIndex.empty')}</p>
      ) : null}
      {visiblePostCards.length > 0 ? (
        <ul className='post-list' aria-label={t('shell:communityIndex.results')}>
          {visiblePostCards.map(({ key, view, resolvedEntry }) => {
            const capabilities = resolvedEntry?.capabilities;
            return (
            <li key={key}>
              <PostCard
                view={view}
                readOnly={!view.actionPost}
                mediaObjectUrls={mediaObjectUrls}
                onOpenAuthor={onOpenAuthor}
                onOpenThread={onOpenThread ?? (() => undefined)}
                onOpenThreadInTopic={onOpenThreadInTopic}
                onReply={onReply ?? (() => undefined)}
                onRepost={capabilities?.repost ? onRepost : undefined}
                onQuoteRepost={capabilities?.quote_repost ? onQuoteRepost : undefined}
                localAuthorPubkey={localAuthorPubkey}
                ownedReactionAssets={ownedReactionAssets}
                bookmarkedReactionAssets={bookmarkedReactionAssets}
                recentReactions={recentReactions}
                onToggleReaction={
                  capabilities?.react && onToggleReaction
                    ? async (post, reactionKey) => {
                        await onToggleReaction(post, reactionKey);
                        await refreshResolvedEntry(key);
                      }
                    : undefined
                }
                onBookmarkCustomReaction={onBookmarkCustomReaction}
                onReactionPickerOpen={onReactionPickerOpen}
                showBookmarkAction={Boolean(capabilities?.bookmark) && showBookmarkAction}
                isBookmarked={bookmarkedPostIds?.has(view.post.object_id) ?? false}
                onToggleBookmark={
                  capabilities?.bookmark && onToggleBookmark
                    ? async (post) => {
                        await onToggleBookmark(post);
                        await refreshResolvedEntry(key);
                      }
                    : undefined
                }
                onWithdraw={
                  capabilities?.withdraw && onWithdraw
                    ? async (post) => {
                        await onWithdraw(post);
                        await refreshResolvedEntry(key);
                      }
                    : undefined
                }
                onActivateReference={onActivateReference}
                onCopyLink={capabilities?.copy_link ? onCopyPostLink : undefined}
                onSubmitReport={(request) => api.submitCommunityNodeReport(request)}
                onCopyReportContact={(value) => void copyTextToClipboard(value)}
                onFetchReportManifest={(baseUrl) => api.fetchCommunityNodeManifest(baseUrl)}
              />
            </li>
            );
          })}
        </ul>
      ) : null}

      {consentGateBaseUrl !== null ? (
        <CommunityNodeConsentDialog
          open
          onOpenChange={(open) => {
            if (!open) {
              closeConsentGate();
            }
          }}
          baseUrl={consentGateBaseUrl}
          consent={communityNodeConsentView(undefined, consentGateEntry ?? undefined)}
          busy={consentGateBusy}
          onAccept={() => void acceptConsentGate()}
          onRetry={() => void openConsentGate(consentGateBaseUrl)}
        />
      ) : null}
    </Card>
  );
}
