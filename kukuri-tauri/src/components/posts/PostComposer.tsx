import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Label } from '@/components/ui/label';
import { Card, CardContent } from '@/components/ui/card';
import { usePostStore } from '@/stores/postStore';
import { useTopicStore } from '@/stores/topicStore';
import { useToast } from '@/hooks/use-toast';
import { Loader2 } from 'lucide-react';

interface PostComposerProps {
  topicId?: string;
  onSuccess?: () => void;
  onCancel?: () => void;
}

export function PostComposer({ topicId, onSuccess, onCancel }: PostComposerProps) {
  const [content, setContent] = useState('');
  const [selectedTopicId, setSelectedTopicId] = useState(topicId || '');
  const [isSubmitting, setIsSubmitting] = useState(false);

  const { createPost } = usePostStore();
  const { topics, joinedTopics } = useTopicStore();
  const { toast } = useToast();

  // 参加しているトピックのみフィルタリング
  const availableTopics = Array.from(topics.values()).filter((topic) =>
    joinedTopics.includes(topic.id),
  );

  const handleSubmit = async () => {
    if (!content.trim()) {
      toast({
        title: 'エラー',
        description: '投稿内容を入力してください',
        variant: 'destructive',
      });
      return;
    }

    if (!selectedTopicId) {
      toast({
        title: 'エラー',
        description: 'トピックを選択してください',
        variant: 'destructive',
      });
      return;
    }

    setIsSubmitting(true);
    try {
      await createPost(content, selectedTopicId);
      toast({
        title: '成功',
        description: '投稿を作成しました',
      });
      setContent('');
      onSuccess?.();
    } catch {
      // エラーハンドリングはストアで行われる
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleCancel = () => {
    setContent('');
    onCancel?.();
  };

  return (
    <Card className="w-full">
      <CardContent className="pt-6">
        <div className="space-y-4">
          <div>
            <Label htmlFor="topic-select">トピック</Label>
            <Select
              value={selectedTopicId}
              onValueChange={setSelectedTopicId}
              disabled={!!topicId || isSubmitting}
            >
              <SelectTrigger id="topic-select" className="w-full">
                <SelectValue placeholder="トピックを選択" />
              </SelectTrigger>
              <SelectContent>
                {availableTopics.length === 0 ? (
                  <div className="p-2 text-sm text-muted-foreground">
                    参加しているトピックがありません
                  </div>
                ) : (
                  availableTopics.map((topic) => (
                    <SelectItem key={topic.id} value={topic.id}>
                      {topic.name}
                    </SelectItem>
                  ))
                )}
              </SelectContent>
            </Select>
          </div>

          <div>
            <Label htmlFor="post-content">投稿内容</Label>
            <Textarea
              id="post-content"
              placeholder="今何を考えていますか？"
              value={content}
              onChange={(e) => setContent(e.target.value)}
              disabled={isSubmitting}
              rows={4}
              className="resize-none"
            />
          </div>

          <div className="flex justify-end gap-2">
            <Button variant="outline" onClick={handleCancel} disabled={isSubmitting}>
              キャンセル
            </Button>
            <Button
              onClick={handleSubmit}
              disabled={isSubmitting || !content.trim() || !selectedTopicId}
            >
              {isSubmitting && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
              投稿する
            </Button>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
