import { useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Expand, Loader2, X } from 'lucide-react';
import { useThreadPosts } from '@/hooks';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { ForumThreadView } from './ForumThreadView';

const DRAG_LEFT_THRESHOLD_PX = 120;
const MAX_DRAG_OFFSET_PX = 220;

const isInteractiveElement = (target: EventTarget | null): boolean => {
  if (!(target instanceof Element)) {
    return false;
  }
  return target.closest('button, a, input, textarea, select, [role="button"]') !== null;
};

interface ThreadPreviewPaneProps {
  topicId: string;
  threadUuid: string;
  onClose: () => void;
  onOpenFullThread: () => void;
}

export function ThreadPreviewPane({
  topicId,
  threadUuid,
  onClose,
  onOpenFullThread,
}: ThreadPreviewPaneProps) {
  const { t } = useTranslation();
  const { data: threadPosts, isLoading, error, refetch } = useThreadPosts(topicId, threadUuid);
  const dragStartXRef = useRef<number | null>(null);
  const thresholdReachedRef = useRef(false);
  const [dragOffsetX, setDragOffsetX] = useState(0);

  const resetDragState = () => {
    dragStartXRef.current = null;
    thresholdReachedRef.current = false;
    setDragOffsetX(0);
  };

  const handlePointerDown = (event: React.PointerEvent<HTMLElement>) => {
    if (isInteractiveElement(event.target)) {
      return;
    }
    if (event.pointerType === 'mouse' && event.button !== 0) {
      return;
    }

    dragStartXRef.current = event.clientX;
    thresholdReachedRef.current = false;
    event.currentTarget.setPointerCapture(event.pointerId);
  };

  const handlePointerMove = (event: React.PointerEvent<HTMLElement>) => {
    if (dragStartXRef.current === null || thresholdReachedRef.current) {
      return;
    }

    const deltaX = event.clientX - dragStartXRef.current;
    const nextOffset = deltaX < 0 ? Math.max(deltaX, -MAX_DRAG_OFFSET_PX) : 0;
    setDragOffsetX(nextOffset);

    if (deltaX <= -DRAG_LEFT_THRESHOLD_PX) {
      thresholdReachedRef.current = true;
      onOpenFullThread();
      resetDragState();
    }
  };

  const handlePointerEnd = (event: React.PointerEvent<HTMLElement>) => {
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
    resetDragState();
  };

  return (
    <aside
      className="h-fit rounded-lg border bg-card p-4"
      data-testid="thread-preview-pane"
      onPointerDown={handlePointerDown}
      onPointerMove={handlePointerMove}
      onPointerUp={handlePointerEnd}
      onPointerCancel={handlePointerEnd}
      style={{ touchAction: 'pan-y' }}
    >
      <div
        className="space-y-4 transition-transform duration-150 ease-out"
        style={{ transform: `translateX(${dragOffsetX}px)` }}
      >
        <header className="space-y-3">
          <div className="flex items-start justify-between gap-3">
            <div>
              <p className="text-sm font-semibold text-foreground">
                {t('topics.threadPreviewTitle')}
              </p>
              <p className="text-xs text-muted-foreground">
                {t('topics.threadDetailUuid', { uuid: threadUuid })}
              </p>
            </div>
            <Button
              type="button"
              variant="ghost"
              size="icon"
              onClick={onClose}
              aria-label={t('topics.closePreview')}
              data-testid="thread-preview-close"
            >
              <X className="h-4 w-4" />
            </Button>
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={onOpenFullThread}
              data-testid="thread-preview-open-full"
            >
              <Expand className="mr-2 h-4 w-4" />
              {t('topics.openThreadFullscreen')}
            </Button>
            <p className="text-xs text-muted-foreground" data-testid="thread-preview-drag-hint">
              {t('topics.threadPreviewDragHint')}
            </p>
          </div>
        </header>

        <section className="max-h-[70vh] overflow-y-auto pr-1">
          {isLoading ? (
            <div className="flex justify-center py-10" data-testid="thread-preview-loading">
              <Loader2 className="h-6 w-6 animate-spin" />
            </div>
          ) : error ? (
            <Alert variant="destructive" data-testid="thread-preview-error">
              <AlertDescription className="space-y-2">
                <p>{t('topics.threadLoadFailed')}</p>
                <Button type="button" variant="outline" size="sm" onClick={() => refetch()}>
                  {t('common.retry')}
                </Button>
              </AlertDescription>
            </Alert>
          ) : !threadPosts || threadPosts.length === 0 ? (
            <Alert data-testid="thread-preview-empty">
              <AlertDescription>{t('topics.threadNotFound')}</AlertDescription>
            </Alert>
          ) : (
            <ForumThreadView threadUuid={threadUuid} posts={threadPosts} />
          )}
        </section>
      </div>
    </aside>
  );
}
