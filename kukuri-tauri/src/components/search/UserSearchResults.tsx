import { useQuery } from '@tanstack/react-query';
import { Card, CardContent } from '@/components/ui/card';
import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar';
import { Button } from '@/components/ui/button';
import { Loader2, UserPlus } from 'lucide-react';
import { Link } from '@tanstack/react-router';
import type { Profile } from '@/stores';

interface UserSearchResultsProps {
  query: string;
}

// 仮のユーザーデータ（実際にはAPIから取得）
const mockUsers: Profile[] = [
  {
    id: '1',
    pubkey: 'pubkey1',
    npub: 'npub1xxx',
    name: 'alice',
    displayName: 'Alice',
    about: 'Nostr開発者',
    picture: '',
    nip05: 'alice@example.com',
  },
  {
    id: '2',
    pubkey: 'pubkey2',
    npub: 'npub2xxx',
    name: 'bob',
    displayName: 'Bob Smith',
    about: 'ビットコインエンスージアスト',
    picture: '',
    nip05: '',
  },
];

export function UserSearchResults({ query }: UserSearchResultsProps) {
  // クライアントサイドでユーザーを検索
  const searchResults = useQuery({
    queryKey: ['search', 'users', query],
    queryFn: async () => {
      if (!query) return [];

      const searchTerm = query.toLowerCase();

      // ユーザー名、表示名、自己紹介で検索
      return mockUsers.filter((user) => {
        const nameMatch = user.name?.toLowerCase().includes(searchTerm) || false;
        const displayNameMatch = user.displayName?.toLowerCase().includes(searchTerm) || false;
        const aboutMatch = user.about?.toLowerCase().includes(searchTerm) || false;
        const nip05Match = user.nip05?.toLowerCase().includes(searchTerm) || false;

        return nameMatch || displayNameMatch || aboutMatch || nip05Match;
      });
    },
    enabled: !!query,
    staleTime: 0, // 常に最新のデータで検索
  });

  if (!query) {
    return (
      <div className="text-center py-12 text-muted-foreground">
        検索キーワードを入力してください
      </div>
    );
  }

  if (searchResults.isLoading) {
    return (
      <div className="flex justify-center py-12">
        <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
      </div>
    );
  }

  const results = searchResults.data || [];

  if (results.length === 0) {
    return (
      <div className="text-center py-12">
        <p className="text-lg font-medium">検索結果が見つかりませんでした</p>
        <p className="text-muted-foreground mt-2">「{query}」に一致するユーザーはいません</p>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <p className="text-sm text-muted-foreground">{results.length}人のユーザーが見つかりました</p>
      <div className="space-y-4">
        {results.map((user) => (
          <UserCard key={user.id} user={user} />
        ))}
      </div>
    </div>
  );
}

function UserCard({ user }: { user: Profile }) {
  const getInitials = (name: string) => {
    return name
      .split(' ')
      .map((n) => n[0])
      .join('')
      .toUpperCase()
      .slice(0, 2);
  };

  return (
    <Card>
      <CardContent className="flex items-center justify-between p-4">
        <div className="flex items-center gap-3">
          <Avatar className="h-12 w-12">
            <AvatarImage src={user.picture} />
            <AvatarFallback>{getInitials(user.displayName || user.name || 'U')}</AvatarFallback>
          </Avatar>
          <div>
            <Link
              to="/profile/$userId"
              params={{ userId: user.id }}
              className="font-semibold hover:underline"
            >
              {user.displayName || user.name || 'ユーザー'}
            </Link>
            {user.nip05 && <p className="text-sm text-muted-foreground">{user.nip05}</p>}
            {user.about && (
              <p className="text-sm text-muted-foreground mt-1 line-clamp-1">{user.about}</p>
            )}
            <p className="text-xs text-muted-foreground mt-1">{user.npub}</p>
          </div>
        </div>
        <Button size="sm" variant="outline">
          <UserPlus className="h-4 w-4 mr-1" />
          フォロー
        </Button>
      </CardContent>
    </Card>
  );
}
