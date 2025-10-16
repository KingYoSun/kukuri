import { useState, useEffect, useCallback, useMemo } from 'react';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';
import { Card, CardContent, CardFooter } from '@/components/ui/card';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { usePostStore } from '@/stores/postStore';
import { useTopicStore } from '@/stores/topicStore';
import { useDraftStore } from '@/stores/draftStore';
import { useToast } from '@/hooks/use-toast';
import { Loader2, FileText, Send, Save, Trash2 } from 'lucide-react';
import { TopicSelector } from '../topics/TopicSelector';
import MarkdownEditor from './MarkdownEditor';
import DraftManager from './DraftManager';
import type { PostDraft } from '@/types/draft';
import { debounce } from 'lodash';
import { errorHandler } from '@/lib/errorHandler';

interface PostComposerProps {
  topicId?: string;
  onSuccess?: () => void;
  onCancel?: () => void;
  replyTo?: string;
  quotedPost?: string;
}

export function PostComposer({
  topicId,
  onSuccess,
  onCancel,
  replyTo,
  quotedPost,
}: PostComposerProps) {
  const [content, setContent] = useState('');
  const [selectedTopicId, setSelectedTopicId] = useState(topicId || '');
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [currentDraftId, setCurrentDraftId] = useState<string | null>(null);
  const [showDrafts, setShowDrafts] = useState(false);
  const [editorMode, setEditorMode] = useState<'simple' | 'markdown'>('simple');

  const { createPost } = usePostStore();
  const { topics } = useTopicStore();
  const { createDraft, deleteDraft, autosaveDraft } = useDraftStore();
  const { toast } = useToast();

  // Get topic name for draft
  const getTopicName = useCallback(() => {
    if (!selectedTopicId) return undefined;
    return topics.get(selectedTopicId)?.name;
  }, [selectedTopicId, topics]);

  // Autosave logic
  const autosave = useCallback(() => {
    if (!content.trim() && !currentDraftId) return;

    if (currentDraftId) {
      // Update existing draft
      autosaveDraft({
        id: currentDraftId,
        content,
        topicId: selectedTopicId || null,
        topicName: getTopicName(),
        metadata: {
          replyTo,
          quotedPost,
        },
      });
    } else if (content.trim()) {
      // Create new draft
      const draft = createDraft({
        content,
        topicId: selectedTopicId || null,
        topicName: getTopicName(),
        metadata: {
          replyTo,
          quotedPost,
        },
      });
      setCurrentDraftId(draft.id);
    }
  }, [
    content,
    selectedTopicId,
    currentDraftId,
    replyTo,
    quotedPost,
    createDraft,
    autosaveDraft,
    getTopicName,
  ]);

  // Debounced autosave
  const debouncedAutosave = useMemo(() => debounce(autosave, 2000), [autosave]);

  // Trigger autosave on content change
  useEffect(() => {
    if (content || currentDraftId) {
      debouncedAutosave();
    }
    return () => {
      debouncedAutosave.cancel();
    };
  }, [content, selectedTopicId, debouncedAutosave, currentDraftId]);

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
      await createPost(content, selectedTopicId, {
        replyTo,
        quotedPost,
      });

      toast({
        title: '成功',
        description: '投稿を作成しました',
      });

      // Clean up
      if (currentDraftId) {
        deleteDraft(currentDraftId);
      }

      resetForm();
      onSuccess?.();
    } catch (error) {
      errorHandler.log('Failed to create post', error, {
        context: 'Failed to create post',
        showToast: true,
      });
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleCancel = () => {
    if (content.trim() && !currentDraftId) {
      // Save as draft before canceling
      autosave();
      toast({
        title: '下書きを保存しました',
        description: '下書き一覧から再開できます',
      });
    }
    resetForm();
    onCancel?.();
  };

  const handleSaveDraft = () => {
    autosave();
    toast({
      title: '下書きを保存しました',
      description: '下書き一覧から編集を再開できます',
    });
  };

  const handleSelectDraft = (draft: PostDraft) => {
    setContent(draft.content);
    setSelectedTopicId(draft.topicId || '');
    setCurrentDraftId(draft.id);
    setShowDrafts(false);
  };

  const handleDeleteCurrentDraft = () => {
    if (currentDraftId) {
      deleteDraft(currentDraftId);
      resetForm();
      toast({
        title: '下書きを削除しました',
      });
    }
  };

  const resetForm = () => {
    setContent('');
    setSelectedTopicId(topicId || '');
    setCurrentDraftId(null);
  };

  const handleImageUpload = async (file: File): Promise<string> => {
    // 画像サイズの制限（5MB）
    const MAX_SIZE = 5 * 1024 * 1024;

    if (file.size > MAX_SIZE) {
      throw new Error('画像サイズは5MB以下にしてください');
    }

    // 画像形式の確認
    if (!file.type.startsWith('image/')) {
      throw new Error('画像ファイルを選択してください');
    }

    return new Promise((resolve, reject) => {
      const reader = new FileReader();

      reader.onload = (e) => {
        const result = e.target?.result;
        if (typeof result === 'string') {
          // データURLとして返す（base64エンコードされた画像）
          resolve(result);
        } else {
          reject(new Error('画像の読み込みに失敗しました'));
        }
      };

      reader.onerror = () => {
        reject(new Error('画像の読み込みに失敗しました'));
      };

      // Base64エンコードされたデータURLとして読み込む
      reader.readAsDataURL(file);
    });
  };

  return (
    <Card className="w-full">
      <CardContent className="pt-6">
        <Tabs value={editorMode} onValueChange={(v) => setEditorMode(v as 'simple' | 'markdown')}>
          <div className="flex items-center justify-between mb-4">
            <TabsList>
              <TabsTrigger value="simple">シンプル</TabsTrigger>
              <TabsTrigger value="markdown">Markdown</TabsTrigger>
            </TabsList>

            <div className="flex items-center gap-2">
              <Button size="sm" variant="ghost" onClick={() => setShowDrafts(!showDrafts)}>
                <FileText className="w-4 h-4 mr-1" />
                下書き
              </Button>

              {currentDraftId && (
                <Button
                  size="sm"
                  variant="ghost"
                  onClick={handleDeleteCurrentDraft}
                  className="text-destructive hover:text-destructive"
                >
                  <Trash2 className="w-4 h-4" />
                </Button>
              )}
            </div>
          </div>

          {showDrafts && (
            <div className="mb-4">
              <DraftManager onSelectDraft={handleSelectDraft} />
            </div>
          )}

          <div className="space-y-4">
            {/* Topic selector */}
            <TopicSelector
              value={selectedTopicId}
              onValueChange={setSelectedTopicId}
              disabled={!!topicId || isSubmitting}
              placeholder="トピックを選択"
            />

            {/* Content editor */}
            <TabsContent value="simple" className="mt-0">
              <Textarea
                placeholder="今何を考えていますか？"
                value={content}
                onChange={(e) => setContent(e.target.value)}
                disabled={isSubmitting}
                rows={6}
                className="resize-none"
                maxLength={1000}
                data-testid="post-input"
              />
              <div className="text-right text-xs text-muted-foreground mt-1">
                {content.length} / 1000
              </div>
            </TabsContent>

            <TabsContent value="markdown" className="mt-0">
              <MarkdownEditor
                value={content}
                onChange={setContent}
                placeholder="Markdownで投稿を書く..."
                height={300}
                preview="live"
                onImageUpload={handleImageUpload}
                maxLength={1000}
              />
            </TabsContent>

            {/* Reply/Quote indicator */}
            {(replyTo || quotedPost) && (
              <div className="text-sm text-muted-foreground bg-muted p-2 rounded">
                {replyTo && <div>返信先: {replyTo}</div>}
                {quotedPost && <div>引用: {quotedPost}</div>}
              </div>
            )}
          </div>
        </Tabs>
      </CardContent>

      <CardFooter className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          {currentDraftId && (
            <span className="text-xs text-muted-foreground">下書きを自動保存中...</span>
          )}
        </div>

        <div className="flex gap-2">
          <Button variant="outline" onClick={handleCancel} disabled={isSubmitting}>
            キャンセル
          </Button>

          <Button
            variant="outline"
            onClick={handleSaveDraft}
            disabled={isSubmitting || !content.trim()}
          >
            <Save className="w-4 h-4 mr-1" />
            下書き保存
          </Button>

          <Button
            onClick={handleSubmit}
            disabled={isSubmitting || !content.trim() || !selectedTopicId}
            data-testid="submit-post-button"
          >
            {isSubmitting ? (
              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
            ) : (
              <Send className="mr-2 h-4 w-4" />
            )}
            投稿する
          </Button>
        </div>
      </CardFooter>
    </Card>
  );
}
