import { createFileRoute } from '@tanstack/react-router';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
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
          ? `レート制限中です (${retryAfterSeconds}秒後に再試行できます)`
          : 'レート制限中です。しばらく待ってから再検索してください';
    } else if (errorKey === 'UserSearch.fetch_failed') {
      validationState = 'error';
      validationMessage = '検索に失敗しました。ネットワークを確認して再試行してください';
    } else if (allowIncompleteActive && helperSearch) {
      validationState = 'warning';
      validationMessage = '補助検索モード: キャッシュ優先で部分一致を表示しています';
    } else if (
      sanitizedQuery.length > 0 &&
      (status === 'typing' || errorKey === 'UserSearch.invalid_query')
    ) {
      validationState = 'warning';
      validationMessage = `検索キーワードは${MIN_USER_SEARCH_QUERY_LENGTH}文字以上入力してください`;
    }

    const helperLabel = helperSearch
      ? helperSearch.kind === 'hashtag'
        ? `タグ補助検索中: #${helperSearch.term}`
        : `npub補助検索中: @${helperSearch.term}`
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
        <h1 className="text-3xl font-bold">検索</h1>
        <p className="text-muted-foreground">投稿、トピック、ユーザーを検索できます</p>
      </div>

      <SearchBar
        placeholder={
          activeTab === 'posts'
            ? '投稿を検索...'
            : activeTab === 'topics'
              ? 'トピックを検索...'
              : 'ユーザーを検索...'
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
            投稿
          </TabsTrigger>
          <TabsTrigger value="topics" data-testid="search-tab-topics">
            トピック
          </TabsTrigger>
          <TabsTrigger value="users" data-testid="search-tab-users">
            ユーザー
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
