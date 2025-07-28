import { Card, CardContent, CardHeader } from '@/components/ui/card';
import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar';
import { Button } from '@/components/ui/button';
import { Heart, MessageCircle, Repeat2, Share } from 'lucide-react';
import type { Post } from '@/stores';
import { formatDistanceToNow } from 'date-fns';
import { ja } from 'date-fns/locale';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { TauriApi } from '@/lib/api/tauri';
import { toast } from 'sonner';

interface PostCardProps {
  post: Post;
}

export function PostCard({ post }: PostCardProps) {
  const queryClient = useQueryClient();
  
  // いいね機能
  const likeMutation = useMutation({
    mutationFn: async () => {
      await TauriApi.likePost(post.id);
    },
    onSuccess: () => {
      // 楽観的UI更新
      queryClient.invalidateQueries({ queryKey: ['timeline'] });
      queryClient.invalidateQueries({ queryKey: ['posts', post.topicId] });
    },
    onError: () => {
      toast.error('いいねに失敗しました');
    },
  });

  const handleLike = () => {
    likeMutation.mutate();
  };

  // 時間表示のフォーマット
  const timeAgo = formatDistanceToNow(new Date(post.created_at * 1000), {
    addSuffix: true,
    locale: ja,
  });

  // アバターのイニシャルを生成
  const getInitials = (name: string) => {
    return name
      .split(' ')
      .map(n => n[0])
      .join('')
      .toUpperCase()
      .slice(0, 2);
  };

  return (
    <Card>
      <CardHeader>
        <div className="flex items-start gap-3">
          <Avatar>
            <AvatarImage src={post.author.picture} />
            <AvatarFallback>
              {getInitials(post.author.displayName || post.author.name || 'U')}
            </AvatarFallback>
          </Avatar>
          <div className="flex-1">
            <div className="flex items-center gap-2">
              <h4 className="font-semibold">
                {post.author.displayName || post.author.name || 'ユーザー'}
              </h4>
              <span className="text-sm text-muted-foreground">{timeAgo}</span>
            </div>
            <p className="text-sm text-muted-foreground">{post.author.npub}</p>
          </div>
        </div>
      </CardHeader>
      <CardContent>
        <p className="mb-4 whitespace-pre-wrap">{post.content}</p>
        <div className="flex items-center gap-6">
          <Button variant="ghost" size="sm" disabled>
            <MessageCircle className="mr-2 h-4 w-4" />
            {post.replies.length}
          </Button>
          <Button variant="ghost" size="sm" disabled>
            <Repeat2 className="mr-2 h-4 w-4" />
            0
          </Button>
          <Button 
            variant="ghost" 
            size="sm"
            onClick={handleLike}
            disabled={likeMutation.isPending}
          >
            <Heart className="mr-2 h-4 w-4" />
            {post.likes}
          </Button>
          <Button variant="ghost" size="sm" aria-label="share" disabled>
            <Share className="h-4 w-4" />
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}