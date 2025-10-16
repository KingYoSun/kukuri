import { Card, CardContent, CardHeader, CardDescription } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Users, MessageSquare, Clock, Hash, Loader2 } from 'lucide-react';
import { useNavigate } from '@tanstack/react-router';
import type { Topic } from '@/stores';
import { formatDistanceToNow } from 'date-fns';
import { ja } from 'date-fns/locale';
import { useTopicStore } from '@/stores';
import { useToast } from '@/hooks/use-toast';
import { useState, useMemo } from 'react';

interface TopicCardProps {
  topic: Topic;
}

export function TopicCard({ topic }: TopicCardProps) {
  const { joinedTopics, joinTopic, leaveTopic, setCurrentTopic } = useTopicStore();
  const navigate = useNavigate();
  // joinedTopicsが変更されたときのみ再計算
  const isJoined = useMemo(() => joinedTopics.includes(topic.id), [joinedTopics, topic.id]);
  const [isLoading, setIsLoading] = useState(false);
  const { toast } = useToast();

  // 最終アクティブ時刻の表示
  const lastActiveText = topic.lastActive
    ? formatDistanceToNow(new Date(topic.lastActive * 1000), {
        addSuffix: true,
        locale: ja,
      })
    : '活動なし';

  const handleTopicClick = () => {
    setCurrentTopic(topic);
    navigate({ to: '/' }); // ホーム（タイムライン）に遷移
  };

  const handleJoinToggle = async () => {
    setIsLoading(true);
    try {
      if (isJoined) {
        // トピックから離脱
        await leaveTopic(topic.id);
        toast({
          title: 'トピックから離脱しました',
          description: `「${topic.name}」から離脱しました`,
        });
      } else {
        // トピックに参加
        await joinTopic(topic.id);
        toast({
          title: 'トピックに参加しました',
          description: `「${topic.name}」に参加しました`,
        });
      }
    } catch {
      toast({
        title: 'エラー',
        description: isJoined
          ? 'トピックから離脱できませんでした'
          : 'トピックに参加できませんでした',
        variant: 'destructive',
      });
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <Card className="hover:shadow-lg transition-shadow">
      <CardHeader>
        <div className="flex items-start justify-between">
          <div>
            <h3
              className="text-lg font-semibold flex items-center gap-2 hover:underline cursor-pointer"
              onClick={handleTopicClick}
            >
              <Hash className="h-4 w-4 text-muted-foreground" />
              {topic.name}
            </h3>
            <CardDescription className="mt-1">{topic.description}</CardDescription>
          </div>
          <Button
            variant={isJoined ? 'secondary' : 'default'}
            size="sm"
            onClick={handleJoinToggle}
            disabled={isLoading}
            aria-pressed={isJoined}
            aria-label={isJoined ? `「${topic.name}」から離脱` : `「${topic.name}」に参加`}
          >
            {isLoading && <Loader2 className="h-3 w-3 mr-2 animate-spin" />}
            {isJoined ? '参加中' : '参加'}
          </Button>
        </div>
      </CardHeader>
      <CardContent>
        <div className="flex items-center gap-4 text-sm text-muted-foreground">
          <div className="flex items-center gap-1">
            <Users className="h-4 w-4" />
            <span>{topic.memberCount} メンバー</span>
          </div>
          <div className="flex items-center gap-1">
            <MessageSquare className="h-4 w-4" />
            <span>{topic.postCount} 投稿</span>
          </div>
          <div className="flex items-center gap-1">
            <Clock className="h-4 w-4" />
            <span>{lastActiveText}</span>
          </div>
        </div>
        {topic.tags.length > 0 && (
          <div className="flex flex-wrap gap-2 mt-3">
            {topic.tags.map((tag) => (
              <Badge key={tag} variant="secondary">
                {tag}
              </Badge>
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
