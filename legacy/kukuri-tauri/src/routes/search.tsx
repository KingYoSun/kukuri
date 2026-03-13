import { createFileRoute } from '@tanstack/react-router';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { SearchBar } from '@/components/search/SearchBar';
import { PostSearchResults } from '@/components/search/PostSearchResults';
import { TopicSearchResults } from '@/components/search/TopicSearchResults';
import { UserSearchResults, type UserSearchInputMeta } from '@/components/search/UserSearchResults';
import { MIN_USER_SEARCH_QUERY_LENGTH } from '@/hooks/useUserSearchQuery';
import { errorHandler } from '@/lib/errorHandler';

export const Route = createFileRoute('/search')({
  component: SearchPage,
});

type SearchTab = 'posts' | 'topics' | 'users';

function SearchPage() {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = useState<SearchTab>('posts');
  const [searchQuery, setSearchQuery] = useState('');
  const [userSearchMeta, setUserSearchMeta] = useState<UserSearchInputMeta | null>(null);
  const rateLimitLoggedRef = useRef(false);
  const allowIncompleteLoggedRef = useRef(false);

  const handleUserSearchMetaChange = useCallback((meta: UserSearchInputMeta) => {
    setUserSearchMeta(meta);
  }, []);

  useEffect(() => {
    if (!userSearchMeta) {
      rateLimitLoggedRef.current = false;
      allowIncompleteLoggedRef.current = false;
      return;
    }
    const { errorKey, sanitizedQuery, helperSearch, retryAfterSeconds, allowIncompleteActive } =
      userSearchMeta;

    if (errorKey === 'UserSearch.rate_limited') {
      if (!rateLimitLoggedRef.current) {
        rateLimitLoggedRef.current = true;
        errorHandler.info('UserSearch.rate_limited', 'SearchPage.userSearch', {
          query: sanitizedQuery,
          helperKind: helperSearch?.kind ?? null,
          retryAfterSeconds: retryAfterSeconds ?? null,
        });
      }
    } else {
      rateLimitLoggedRef.current = false;
    }

    const allowActive = Boolean(allowIncompleteActive);
    if (allowActive !== allowIncompleteLoggedRef.current) {
      allowIncompleteLoggedRef.current = allowActive;
      const metadata = {
        query: helperSearch?.rawQuery ?? sanitizedQuery,
        helperKind: helperSearch?.kind ?? null,
      };
      errorHandler.info(
        allowActive ? 'UserSearch.allow_incomplete_enabled' : 'UserSearch.allow_incomplete_cleared',
        'SearchPage.userSearch',
        metadata,
      );
    }
  }, [userSearchMeta]);

  const userValidation = useMemo(() => {
    if (!userSearchMeta) {
      return {
        validationState: 'default' as const,
        validationMessage: undefined,
        helperLabel: undefined,
      };
    }

    const {
      sanitizedQuery,
      status,
      errorKey,
      helperSearch,
      allowIncompleteActive,
      retryAfterSeconds,
    } = userSearchMeta;
    let validationState: 'default' | 'warning' | 'error' = 'default';
    let validationMessage: string | undefined;

    if (errorKey === 'UserSearch.rate_limited') {
      validationState = 'error';
      validationMessage =
        retryAfterSeconds && retryAfterSeconds > 0
          ? t('search.rateLimitedRetry', { seconds: retryAfterSeconds })
          : t('search.rateLimitedWait');
    } else if (errorKey === 'UserSearch.fetch_failed') {
      validationState = 'error';
      validationMessage = t('search.fetchFailed');
    } else if (allowIncompleteActive && helperSearch) {
      validationState = 'warning';
      validationMessage = t('search.helperSearchMode');
    } else if (
      sanitizedQuery.length > 0 &&
      (status === 'typing' || errorKey === 'UserSearch.invalid_query')
    ) {
      validationState = 'warning';
      validationMessage = t('search.minQueryLength', { minLength: MIN_USER_SEARCH_QUERY_LENGTH });
    }

    const helperLabel = helperSearch
      ? helperSearch.kind === 'hashtag'
        ? t('search.hashtagHelperSearch', { term: helperSearch.term })
        : t('search.npubHelperSearch', { term: helperSearch.term })
      : undefined;

    return { validationState, validationMessage, helperLabel };
  }, [userSearchMeta]);

  const searchBarValidationState =
    activeTab === 'users' ? userValidation.validationState : 'default';
  const searchBarValidationMessage =
    activeTab === 'users' ? userValidation.validationMessage : undefined;
  const searchBarHelperLabel = activeTab === 'users' ? userValidation.helperLabel : undefined;

  return (
    <div className="max-w-4xl mx-auto space-y-6" data-testid="search-page">
      <div className="space-y-2">
        <h1 className="text-3xl font-bold">{t('search.title')}</h1>
        <p className="text-muted-foreground">{t('search.description')}</p>
      </div>

      <SearchBar
        placeholder={
          activeTab === 'posts'
            ? t('search.searchPosts')
            : activeTab === 'topics'
              ? t('search.searchTopics')
              : t('search.searchUsers')
        }
        value={searchQuery}
        onChange={setSearchQuery}
        showButton={false}
        autoFocus
        validationState={searchBarValidationState}
        validationMessage={searchBarValidationMessage}
        helperLabel={searchBarHelperLabel}
      />

      <Tabs value={activeTab} onValueChange={(value) => setActiveTab(value as SearchTab)}>
        <TabsList className="grid w-full grid-cols-3">
          <TabsTrigger value="posts" data-testid="search-tab-posts">
            {t('search.posts')}
          </TabsTrigger>
          <TabsTrigger value="topics" data-testid="search-tab-topics">
            {t('search.topics')}
          </TabsTrigger>
          <TabsTrigger value="users" data-testid="search-tab-users">
            {t('search.users')}
          </TabsTrigger>
        </TabsList>

        <TabsContent value="posts" className="mt-6">
          <PostSearchResults query={searchQuery} />
        </TabsContent>

        <TabsContent value="topics" className="mt-6">
          <TopicSearchResults query={searchQuery} />
        </TabsContent>

        <TabsContent value="users" className="mt-6">
          <UserSearchResults query={searchQuery} onInputMetaChange={handleUserSearchMetaChange} />
        </TabsContent>
      </Tabs>
    </div>
  );
}
