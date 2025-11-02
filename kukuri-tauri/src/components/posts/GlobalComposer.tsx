import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { useComposerStore } from '@/stores/composerStore';
import { PostComposer } from './PostComposer';

export function GlobalComposer() {
  const { isOpen, topicId, replyTo, quotedPost, closeComposer, complete } = useComposerStore();

  if (!isOpen) {
    return null;
  }

  return (
    <Dialog open={isOpen} onOpenChange={(open) => !open && closeComposer()}>
      <DialogContent className="max-w-2xl space-y-4">
        <DialogHeader>
          <DialogTitle>新規投稿</DialogTitle>
        </DialogHeader>
        <PostComposer
          topicId={topicId ?? undefined}
          replyTo={replyTo ?? undefined}
          quotedPost={quotedPost ?? undefined}
          onSuccess={complete}
          onCancel={closeComposer}
        />
      </DialogContent>
    </Dialog>
  );
}
