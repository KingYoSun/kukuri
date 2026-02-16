import { useState, useEffect, useCallback, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';
import { Card, CardContent, CardFooter } from '@/components/ui/card';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
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
import { TopicFormModal } from '@/components/topics/TopicFormModal';
import type { Topic } from '@/stores';
import { useComposerStore } from '@/stores/composerStore';
import { useCommunityNodeStore } from '@/stores/communityNodeStore';
import type { PostScope } from '@/stores/types';

interface PostComposerProps {
  topicId?: string;
  onSuccess?: () => void;
  onCancel?: () => void;
  replyTo?: string;
  quotedPost?: string;
}

const getScopeOptions = (
  t: (key: string) => string,
): Array<{ value: PostScope; label: string; description: string }> => [
  {
    value: 'public',
    label: t('posts.composer.scope.public'),
    description: t('posts.composer.scope.publicDescription'),
  },
  {
    value: 'friend_plus',
    label: t('posts.composer.scope.friend_plus'),
    description: t('posts.composer.scope.friend_plusDescription'),
  },
  {
    value: 'friend',
    label: t('posts.composer.scope.friend'),
    description: t('posts.composer.scope.friendDescription'),
  },
  {
    value: 'invite',
    label: t('posts.composer.scope.invite'),
    description: t('posts.composer.scope.inviteDescription'),
  },
];

export function PostComposer({
  topicId,
  onSuccess,
  onCancel,
  replyTo,
  quotedPost,
}: PostComposerProps) {
  const { t } = useTranslation();
  const [content, setContent] = useState('');
  const [selectedTopicId, setSelectedTopicId] = useState(topicId || '');
  const [selectedScope, setSelectedScope] = useState<PostScope>('public');
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [currentDraftId, setCurrentDraftId] = useState<string | null>(null);
  const [showDrafts, setShowDrafts] = useState(false);
  const [editorMode, setEditorMode] = useState<'simple' | 'markdown'>('simple');
  const [showTopicCreationDialog, setShowTopicCreationDialog] = useState(false);

  const { createPost } = usePostStore();
  const { topics } = useTopicStore();
  const { createDraft, deleteDraft, autosaveDraft } = useDraftStore();
  const { toast } = useToast();
  const applyTopicAndResume = useComposerStore((state) => state.applyTopicAndResume);
  const enableAccessControl = useCommunityNodeStore((state) => state.enableAccessControl);
  const scopeOptions = getScopeOptions(t);

  useEffect(() => {
    if (topicId) {
      setSelectedTopicId(topicId);
    }
  }, [topicId]);

  useEffect(() => {
    if (!enableAccessControl) {
      setSelectedScope('public');
    }
  }, [enableAccessControl]);

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
          scope: selectedScope,
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
          scope: selectedScope,
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
    selectedScope,
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
        title: t('common.error'),
        description: t('posts.composer.contentRequired'),
        variant: 'destructive',
      });
      return;
    }

    if (!selectedTopicId) {
      toast({
        title: t('common.error'),
        description: t('posts.composer.topicRequired'),
        variant: 'destructive',
      });
      return;
    }

    setIsSubmitting(true);
    try {
      const createdPost = await createPost(content, selectedTopicId, {
        replyTo,
        quotedPost,
        scope: selectedScope,
      });
      const isSynced = createdPost?.isSynced !== false;

      toast({
        title: t('common.success'),
        description: isSynced ? t('posts.composer.postCreated') : t('posts.composer.postQueued'),
      });

      // Clean up
      if (currentDraftId) {
        deleteDraft(currentDraftId);
      }

      resetForm();
      if (!isSynced) {
        onCancel?.();
        if (!onCancel) {
          onSuccess?.();
        }
        return;
      }
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
        title: t('posts.composer.draftSaved'),
        description: t('posts.composer.draftSavedDescription'),
      });
    }
    resetForm();
    onCancel?.();
  };

  const handleSaveDraft = () => {
    autosave();
    toast({
      title: t('posts.composer.draftSaved'),
      description: t('posts.composer.draftSavedEditDescription'),
    });
  };

  const handleSelectDraft = (draft: PostDraft) => {
    setContent(draft.content);
    setSelectedTopicId(draft.topicId || '');
    setSelectedScope(draft.metadata?.scope ?? 'public');
    setCurrentDraftId(draft.id);
    setShowDrafts(false);
  };

  const handleDeleteCurrentDraft = () => {
    if (currentDraftId) {
      deleteDraft(currentDraftId);
      resetForm();
      toast({
        title: t('posts.composer.draftDeleted'),
      });
    }
  };

  const resetForm = () => {
    setContent('');
    setSelectedTopicId(topicId || '');
    setSelectedScope('public');
    setCurrentDraftId(null);
  };

  const handleImageUpload = async (file: File): Promise<string> => {
    // 画像サイズの制限（5MB）
    const MAX_SIZE = 5 * 1024 * 1024;

    if (file.size > MAX_SIZE) {
      throw new Error(t('posts.composer.imageSizeLimit'));
    }

    // 画像形式の確認
    if (!file.type.startsWith('image/')) {
      throw new Error(t('posts.composer.imageFileRequired'));
    }

    return new Promise((resolve, reject) => {
      const reader = new FileReader();

      reader.onload = (e) => {
        const result = e.target?.result;
        if (typeof result === 'string') {
          // データURLとして返す（base64エンコードされた画像）
          resolve(result);
        } else {
          reject(new Error(t('posts.composer.imageLoadFailed')));
        }
      };

      reader.onerror = () => {
        reject(new Error(t('posts.composer.imageLoadFailed')));
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
              <TabsTrigger value="simple" data-testid="composer-tab-simple">
                {t('posts.composer.simpleMode')}
              </TabsTrigger>
              <TabsTrigger value="markdown" data-testid="composer-tab-markdown">
                {t('posts.composer.markdownMode')}
              </TabsTrigger>
            </TabsList>

            <div className="flex items-center gap-2">
              <Button
                size="sm"
                variant="ghost"
                onClick={() => setShowDrafts(!showDrafts)}
                data-testid="drafts-toggle"
              >
                <FileText className="w-4 h-4 mr-1" />
                {t('posts.composer.drafts')}
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
            <div className="mb-4" data-testid="drafts-panel">
              <DraftManager onSelectDraft={handleSelectDraft} />
            </div>
          )}

          <div className="space-y-4">
            {/* Topic selector */}
            <TopicSelector
              value={selectedTopicId}
              onValueChange={setSelectedTopicId}
              disabled={!!topicId || isSubmitting}
              placeholder={t('posts.composer.topicPlaceholder')}
              onCreateTopicRequest={topicId ? undefined : () => setShowTopicCreationDialog(true)}
              dataTestId="topic-selector"
            />

            {enableAccessControl && (
              <div className="space-y-2">
                <Label htmlFor="post-scope">{t('posts.composer.scopeLabel')}</Label>
                <Select
                  value={selectedScope}
                  onValueChange={(value) => setSelectedScope(value as PostScope)}
                  disabled={isSubmitting}
                >
                  <SelectTrigger id="post-scope" data-testid="scope-selector">
                    <SelectValue placeholder={t('posts.composer.scopePlaceholder')} />
                  </SelectTrigger>
                  <SelectContent>
                    {scopeOptions.map((option) => (
                      <SelectItem key={option.value} value={option.value}>
                        {option.label}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
                <p className="text-xs text-muted-foreground">
                  {scopeOptions.find((option) => option.value === selectedScope)?.description}
                </p>
              </div>
            )}

            {/* Content editor */}
            <TabsContent value="simple" className="mt-0">
              <Textarea
                placeholder={t('posts.composer.contentPlaceholder')}
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

            <TabsContent value="markdown" className="mt-0" data-testid="markdown-editor-pane">
              <MarkdownEditor
                value={content}
                onChange={setContent}
                placeholder={t('posts.composer.markdownPlaceholder')}
                height={300}
                preview="live"
                onImageUpload={handleImageUpload}
                maxLength={1000}
              />
            </TabsContent>

            {/* Reply/Quote indicator */}
            {(replyTo || quotedPost) && (
              <div className="text-sm text-muted-foreground bg-muted p-2 rounded">
                {replyTo && (
                  <div>
                    {t('posts.composer.replyingTo')}: {replyTo}
                  </div>
                )}
                {quotedPost && (
                  <div>
                    {t('posts.composer.quoting')}: {quotedPost}
                  </div>
                )}
              </div>
            )}
          </div>
        </Tabs>
      </CardContent>

      <CardFooter className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          {currentDraftId && (
            <span className="text-xs text-muted-foreground">{t('posts.composer.autosaving')}</span>
          )}
        </div>

        <div className="flex gap-2">
          <Button variant="outline" onClick={handleCancel} disabled={isSubmitting}>
            {t('posts.cancel')}
          </Button>

          <Button
            variant="outline"
            onClick={handleSaveDraft}
            disabled={isSubmitting || !content.trim()}
            data-testid="save-draft-button"
          >
            <Save className="w-4 h-4 mr-1" />
            {t('posts.composer.saveDraft')}
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
            {t('posts.composer.submit')}
          </Button>
        </div>
      </CardFooter>
      {!topicId && (
        <TopicFormModal
          open={showTopicCreationDialog}
          onOpenChange={setShowTopicCreationDialog}
          mode="create-from-composer"
          autoJoin
          onCreated={(newTopic: Topic) => {
            setSelectedTopicId(newTopic.id);
            applyTopicAndResume(newTopic.id);
            setShowTopicCreationDialog(false);
          }}
        />
      )}
    </Card>
  );
}
