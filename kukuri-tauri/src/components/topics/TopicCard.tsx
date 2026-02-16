import { useTranslation } from 'react-i18next';
import { Card, CardContent, CardHeader, CardDescription } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Users, MessageSquare, Clock, Hash, Loader2 } from 'lucide-react';
import { useNavigate } from '@tanstack/react-router';
import type { Topic } from '@/stores';
import { formatDistanceToNow } from 'date-fns';
import { useTopicStore } from '@/stores';
import { useToast } from '@/hooks/use-toast';
import { useState, useMemo } from 'react';
import { getDateFnsLocale } from '@/i18n';
import { DEFAULT_PUBLIC_TOPIC_ID } from '@/constants/topics';
import i18n from '@/i18n';

interface TopicCardProps {
  topic: Topic;
}

export function TopicCard({ topic }: TopicCardProps) {
  const { t } = useTranslation();
  const { joinedTopics, joinTopic, leaveTopic, setCurrentTopic } = useTopicStore();
  const navigate = useNavigate();
  const isJoined = useMemo(() => joinedTopics.includes(topic.id), [joinedTopics, topic.id]);
  const [isLoading, setIsLoading] = useState(false);
  const { toast } = useToast();

  const displayDescription = topic.id === DEFAULT_PUBLIC_TOPIC_ID
    ? i18n.t('topics.publicTimeline')
    : topic.description;

  const lastActiveText = topic.lastActive
    ? formatDistanceToNow(new Date(topic.lastActive * 1000), {
        addSuffix: true,
        locale: getDateFnsLocale(),
      })
    : t('topics.noActivity');

  const handleTopicClick = () => {
    setCurrentTopic(topic);
    navigate({ to: '/topics/$topicId', params: { topicId: topic.id } }); // Navigate to topic detail
  };

  const handleJoinToggle = async () => {
    setIsLoading(true);
    try {
      if (isJoined) {
        await leaveTopic(topic.id);
        toast({
          title: t('topics.leaveSuccess'),
          description: `「${topic.name}」`,
        });
      } else {
        await joinTopic(topic.id);
        toast({
          title: t('topics.joinSuccess'),
          description: `「${topic.name}」`,
        });
      }
    } catch {
      toast({
        title: t('common.error'),
        description: isJoined ? t('topics.leaveFailed') : t('topics.joinFailed'),
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
            <CardDescription className="mt-1">{displayDescription}</CardDescription>
          </div>
          <Button
            variant={isJoined ? 'secondary' : 'default'}
            size="sm"
            onClick={handleJoinToggle}
            disabled={isLoading}
            aria-pressed={isJoined}
            aria-label={isJoined ? `「${topic.name}」 ${t('topics.leave')}` : `「${topic.name}」 ${t('topics.join')}`}
          >
            {isLoading && <Loader2 className="h-3 w-3 mr-2 animate-spin" />}
            {isJoined ? t('topics.joined') : t('topics.join')}
          </Button>
        </div>
      </CardHeader>
      <CardContent>
        <div className="flex items-center gap-4 text-sm text-muted-foreground">
          <div className="flex items-center gap-1">
            <Users className="h-4 w-4" />
            <span>{t('topics.members', { count: topic.memberCount })}</span>
          </div>
          <div className="flex items-center gap-1">
            <MessageSquare className="h-4 w-4" />
            <span>{t('topics.postsCount', { count: topic.postCount })}</span>
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
