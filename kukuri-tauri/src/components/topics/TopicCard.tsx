import { Card, CardContent, CardHeader, CardDescription } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Users, MessageSquare, Clock, Hash } from 'lucide-react';
import { Link } from '@tanstack/react-router';
import type { Topic } from '@/stores';
import { formatDistanceToNow } from 'date-fns';
import { ja } from 'date-fns/locale';
import { useTopicStore } from '@/stores';

interface TopicCardProps {
  topic: Topic;
}

export function TopicCard({ topic }: TopicCardProps) {
  const { joinedTopics, joinTopic, leaveTopic } = useTopicStore();
  const isJoined = joinedTopics.includes(topic.id);

  // 最終アクティブ時刻の表示
  const lastActiveText = topic.lastActive
    ? formatDistanceToNow(new Date(topic.lastActive * 1000), {
        addSuffix: true,
        locale: ja,
      })
    : '活動なし';

  const handleJoinToggle = () => {
    if (isJoined) {
      leaveTopic(topic.id);
    } else {
      joinTopic(topic.id);
    }
  };

  return (
    <Card className="hover:shadow-lg transition-shadow">
      <CardHeader>
        <div className="flex items-start justify-between">
          <div>
            <Link to="/topics/$topicId" params={{ topicId: topic.id }} className="hover:underline">
              <h3 className="text-lg font-semibold flex items-center gap-2">
                <Hash className="h-4 w-4 text-muted-foreground" />
                {topic.name}
              </h3>
            </Link>
            <CardDescription className="mt-1">{topic.description}</CardDescription>
          </div>
          <Button variant={isJoined ? 'secondary' : 'default'} size="sm" onClick={handleJoinToggle}>
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
