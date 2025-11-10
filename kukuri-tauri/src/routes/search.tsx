import { createFileRoute } from '@tanstack/react-router';
import { useCallback, useMemo, useState } from 'react';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { SearchBar } from '@/components/search/SearchBar';
import { PostSearchResults } from '@/components/search/PostSearchResults';
import { TopicSearchResults } from '@/components/search/TopicSearchResults';
import { UserSearchResults } from '@/components/search/UserSearchResults';
import {
  MIN_USER_SEARCH_QUERY_LENGTH,
  type HelperSearchDescriptor,
  type UserSearchErrorKey,
  type UserSearchStatus,
} from '@/hooks/useUserSearchQuery';

export const Route = createFileRoute('/search')({
  component: SearchPage,
});

type SearchTab = 'posts' | 'topics' | 'users';

interface UserSearchInputMeta {
  sanitizedQuery: string;
  status: UserSearchStatus;
  errorKey: UserSearchErrorKey | null;
  helperSearch: HelperSearchDescriptor | null;
  allowIncompleteActive: boolean;
}

function SearchPage() {
  const [activeTab, setActiveTab] = useState<SearchTab>('posts');
  const [searchQuery, setSearchQuery] = useState('');
  const [userSearchMeta, setUserSearchMeta] = useState<UserSearchInputMeta | null>(null);

  const handleUserSearchMetaChange = useCallback((meta: UserSearchInputMeta) => {
    setUserSearchMeta(meta);
  }, []);

  const userValidation = useMemo(() => {
    if (!userSearchMeta) {
      return {
        validationState: 'default' as const,
        validationMessage: undefined,
        helperLabel: undefined,
      };
    }

    const { sanitizedQuery, status, errorKey, helperSearch, allowIncompleteActive } =
      userSearchMeta;
    let validationState: 'default' | 'warning' | 'error' = 'default';
    let validationMessage: string | undefined;

    if (allowIncompleteActive && helperSearch) {
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
    <div className="max-w-4xl mx-auto space-y-6">
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
          <TabsTrigger value="posts">投稿</TabsTrigger>
          <TabsTrigger value="topics">トピック</TabsTrigger>
          <TabsTrigger value="users">ユーザー</TabsTrigger>
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
