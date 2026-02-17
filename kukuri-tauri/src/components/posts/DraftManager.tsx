import React from 'react';
import { useTranslation } from 'react-i18next';
import { useDraftStore } from '@/stores/draftStore';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardFooter, CardHeader, CardTitle } from '@/components/ui/card';
import { ScrollArea } from '@/components/ui/scroll-area';
import { cn } from '@/lib/utils';
import { format } from 'date-fns';
import { getDateFnsLocale, getCurrentLocale } from '@/i18n';
import { FileText, Clock, Tag, Trash2, Edit } from 'lucide-react';
import type { PostDraft } from '@/types/draft';
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog';

interface DraftManagerProps {
  onSelectDraft: (draft: PostDraft) => void;
  className?: string;
}

const DraftManager: React.FC<DraftManagerProps> = ({ onSelectDraft, className }) => {
  const { t } = useTranslation();
  const { drafts, deleteDraft, clearAllDrafts } = useDraftStore();
  const [deleteConfirmId, setDeleteConfirmId] = React.useState<string | null>(null);
  const [clearAllConfirm, setClearAllConfirm] = React.useState(false);

  const handleDelete = (id: string) => {
    deleteDraft(id);
    setDeleteConfirmId(null);
  };

  const handleClearAll = () => {
    clearAllDrafts();
    setClearAllConfirm(false);
  };

  const formatDate = (date: Date) => {
    const locale = getCurrentLocale();
    const formatStr = locale === 'ja' ? 'M月d日 HH:mm' : locale === 'zh-CN' ? 'M月d日 HH:mm' : 'MMM d, HH:mm';
    return format(new Date(date), formatStr, { locale: getDateFnsLocale() });
  };

  const getPreview = (content: string, maxLength = 100) => {
    const preview = content.replace(/\n/g, ' ').trim();
    return preview.length > maxLength ? preview.substring(0, maxLength) + '...' : preview;
  };

  if (drafts.length === 0) {
    return (
      <Card className={cn('text-center', className)}>
        <CardContent className="py-12">
          <FileText className="w-12 h-12 mx-auto mb-4 text-muted-foreground" />
          <p className="text-muted-foreground">{t('drafts.empty')}</p>
        </CardContent>
      </Card>
    );
  }

  return (
    <>
      <Card className={className}>
        <CardHeader className="pb-3">
          <div className="flex items-center justify-between">
            <CardTitle className="text-lg">{t('drafts.list')}</CardTitle>
            <Button
              size="sm"
              variant="ghost"
              onClick={() => setClearAllConfirm(true)}
              className="text-destructive hover:text-destructive"
            >
              {t('drafts.clearAll')}
            </Button>
          </div>
        </CardHeader>
        <CardContent className="p-0">
          <ScrollArea className="h-[400px]">
            <div className="space-y-2 p-4">
              {drafts.map((draft) => (
                <Card
                  key={draft.id}
                  className="cursor-pointer hover:bg-accent/50 transition-colors"
                  onClick={() => onSelectDraft(draft)}
                >
                  <CardContent className="p-4">
                    <div className="space-y-2">
                      {/* Content preview */}
                      <p className="text-sm line-clamp-2">
                        {getPreview(draft.content) || t('drafts.noContent')}
                      </p>

                      {/* Metadata */}
                      <div className="flex flex-wrap gap-2 text-xs text-muted-foreground">
                        {draft.topicName && (
                          <div className="flex items-center gap-1">
                            <Tag className="w-3 h-3" />
                            <span>{draft.topicName}</span>
                          </div>
                        )}

                        <div className="flex items-center gap-1 ml-auto">
                          <Clock className="w-3 h-3" />
                          <span>{t('drafts.updated')}: {formatDate(draft.updatedAt)}</span>
                        </div>
                      </div>
                    </div>
                  </CardContent>
                  <CardFooter className="p-2 pt-0">
                    <div className="flex justify-end gap-1">
                      <Button
                        size="sm"
                        variant="ghost"
                        onClick={(e) => {
                          e.stopPropagation();
                          onSelectDraft(draft);
                        }}
                      >
                        <Edit className="w-4 h-4" />
                      </Button>
                      <Button
                        size="sm"
                        variant="ghost"
                        onClick={(e) => {
                          e.stopPropagation();
                          setDeleteConfirmId(draft.id);
                        }}
                        className="text-destructive hover:text-destructive"
                      >
                        <Trash2 className="w-4 h-4" />
                      </Button>
                    </div>
                  </CardFooter>
                </Card>
              ))}
            </div>
          </ScrollArea>
        </CardContent>
      </Card>

      {/* Delete confirmation dialog */}
      <AlertDialog
        open={deleteConfirmId !== null}
        onOpenChange={(open) => !open && setDeleteConfirmId(null)}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>{t('drafts.deleteTitle')}</AlertDialogTitle>
            <AlertDialogDescription>
              {t('drafts.deleteDescription')}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>{t('common.cancel')}</AlertDialogCancel>
            <AlertDialogAction
              onClick={() => deleteConfirmId && handleDelete(deleteConfirmId)}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
              {t('common.delete')}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>

      {/* Clear all confirmation dialog */}
      <AlertDialog open={clearAllConfirm} onOpenChange={setClearAllConfirm}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>{t('drafts.clearAllTitle')}</AlertDialogTitle>
            <AlertDialogDescription>
              {t('drafts.clearAllDescription')}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>{t('common.cancel')}</AlertDialogCancel>
            <AlertDialogAction
              onClick={handleClearAll}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
              {t('drafts.clearAll')}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
};

export default DraftManager;
