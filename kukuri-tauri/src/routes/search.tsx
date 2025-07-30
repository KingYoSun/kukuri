import { createFileRoute } from '@tanstack/react-router';
import { useState } from 'react';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { SearchBar } from '@/components/search/SearchBar';
import { PostSearchResults } from '@/components/search/PostSearchResults';
import { TopicSearchResults } from '@/components/search/TopicSearchResults';
import { UserSearchResults } from '@/components/search/UserSearchResults';

export const Route = createFileRoute('/search')({
  component: SearchPage,
});

type SearchTab = 'posts' | 'topics' | 'users';

function SearchPage() {
  const [activeTab, setActiveTab] = useState<SearchTab>('posts');
  const [searchQuery, setSearchQuery] = useState('');

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
          <UserSearchResults query={searchQuery} />
        </TabsContent>
      </Tabs>
    </div>
  );
}
