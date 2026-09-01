import type { PostView } from '@/lib/api';
import type { DraftMediaItem } from '@/shell/slices/shared';
import type { ColumnScope } from '@/shell/slices/workspace';

export type ColumnDraftAction = 'post' | 'reply' | 'message';

export type ColumnDraftTarget = {
  columnId: string;
  action: ColumnDraftAction;
  scope?: ColumnScope;
  threadId?: string;
  peerPubkey?: string;
};

export type ColumnDraftState = ColumnDraftTarget & {
  content: string;
  mediaItems: DraftMediaItem[];
  replyTarget: PostView | null;
  repostTarget: PostView | null;
  // #858: 成人向けとして自己申告して投稿する(content_labels: ['adult'])。
  adultLabeled: boolean;
  expanded: boolean;
  error: string | null;
  pending: boolean;
  attachmentInputKey: number;
};

export type ColumnDraftsSliceState = {
  columnDraftsByKey: Record<string, ColumnDraftState>;
};

function keyPart(value: string | null | undefined) {
  return value ? encodeURIComponent(value) : '-';
}

export function columnDraftKey(target: ColumnDraftTarget): string {
  return [
    'draft',
    keyPart(target.columnId),
    target.action,
    keyPart(target.scope?.topicId),
    keyPart(target.scope?.channelId),
    keyPart(target.threadId),
    keyPart(target.peerPubkey),
  ].join(':');
}

export function createColumnDraft(target: ColumnDraftTarget): ColumnDraftState {
  return {
    ...target,
    content: '',
    mediaItems: [],
    replyTarget: null,
    repostTarget: null,
    adultLabeled: false,
    expanded: false,
    error: null,
    pending: false,
    attachmentInputKey: 0,
  };
}

export function setColumnDraft(
  drafts: Record<string, ColumnDraftState>,
  target: ColumnDraftTarget,
  update: (draft: ColumnDraftState) => ColumnDraftState
): Record<string, ColumnDraftState> {
  const key = columnDraftKey(target);
  const current = drafts[key] ?? createColumnDraft(target);
  const next = update(current);
  if (Object.is(current, next)) return drafts;
  return { ...drafts, [key]: next };
}

export function removeColumnDraft(
  drafts: Record<string, ColumnDraftState>,
  target: ColumnDraftTarget
): Record<string, ColumnDraftState> {
  const key = columnDraftKey(target);
  if (!(key in drafts)) return drafts;
  const next = { ...drafts };
  delete next[key];
  return next;
}

export function isColumnDraftDirty(draft: ColumnDraftState): boolean {
  return Boolean(
    draft.content.trim() || draft.mediaItems.length > 0 || draft.replyTarget || draft.repostTarget
  );
}

export function createInitialColumnDraftsSlice(): ColumnDraftsSliceState {
  return { columnDraftsByKey: {} };
}
