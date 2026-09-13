import {
  columnDraftKey,
  createColumnDraft,
  type ColumnDraftAction,
  type ColumnDraftState,
  type ColumnDraftTarget,
} from '@/shell/slices/columnDrafts';
import type { DesktopShellStoreApi } from '@/shell/store';

export const COLUMN_DRAFT_STORAGE_KEY = 'kukuri:column-drafts:v1';
const COLUMN_DRAFT_STORAGE_VERSION = 1;
const DRAFT_WRITE_DELAY_MS = 250;
const MAX_DRAFT_CONTENT_LENGTH = 200_000;

export type ColumnDraftStorage = Pick<Storage, 'getItem' | 'setItem'>;

const activeDraftWriters = new WeakMap<ColumnDraftStorage, Set<() => () => void>>();

/** Flush the current store synchronously, then fence its debounce/pagehide
 * writer until a failed transition explicitly resumes the same account. */
export function suspendColumnDraftPersistence(storage: ColumnDraftStorage): () => void {
  const resumes: Array<() => void> = [];
  try {
    for (const suspend of activeDraftWriters.get(storage) ?? []) resumes.push(suspend());
  } catch (error) { resumes.forEach((resume) => resume()); throw error; }
  return () => resumes.forEach((resume) => resume());
}

type PersistedColumnDraft = {
  target: ColumnDraftTarget;
  content: string;
  expanded: boolean;
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function parseTarget(value: unknown): ColumnDraftTarget | null {
  if (!isRecord(value) || typeof value.columnId !== 'string' || !value.columnId) return null;
  if (value.action !== 'post' && value.action !== 'reply' && value.action !== 'message') return null;
  const target: ColumnDraftTarget = {
    columnId: value.columnId,
    action: value.action as ColumnDraftAction,
  };
  if (value.scope !== undefined) {
    if (
      !isRecord(value.scope) ||
      typeof value.scope.topicId !== 'string' ||
      !value.scope.topicId ||
      (value.scope.channelId !== null && typeof value.scope.channelId !== 'string')
    ) return null;
    target.scope = { topicId: value.scope.topicId, channelId: value.scope.channelId };
  }
  if (typeof value.threadId === 'string' && value.threadId) target.threadId = value.threadId;
  if (typeof value.peerPubkey === 'string' && value.peerPubkey) target.peerPubkey = value.peerPubkey;
  if (target.action === 'post' && !target.scope) return null;
  if (target.action === 'reply' && (!target.scope || !target.threadId)) return null;
  if (target.action === 'message' && !target.peerPubkey) return null;
  return target;
}

function persistedDrafts(drafts: Record<string, ColumnDraftState>): PersistedColumnDraft[] {
  return Object.values(drafts).flatMap((draft) => {
    if (!draft.content || draft.content.length > MAX_DRAFT_CONTENT_LENGTH) return [];
    const target: ColumnDraftTarget = {
      columnId: draft.columnId,
      action: draft.action,
      ...(draft.scope ? { scope: draft.scope } : {}),
      ...(draft.threadId ? { threadId: draft.threadId } : {}),
      ...(draft.peerPubkey ? { peerPubkey: draft.peerPubkey } : {}),
    };
    return [{ target, content: draft.content, expanded: draft.expanded }];
  });
}

function serializeColumnDrafts(drafts: Record<string, ColumnDraftState>) {
  return JSON.stringify({
    version: COLUMN_DRAFT_STORAGE_VERSION,
    drafts: persistedDrafts(drafts),
  });
}

export function readColumnDrafts(
  storage: ColumnDraftStorage
): Record<string, ColumnDraftState> {
  try {
    const raw = storage.getItem(COLUMN_DRAFT_STORAGE_KEY);
    if (!raw) return {};
    const parsed: unknown = JSON.parse(raw);
    if (
      !isRecord(parsed) ||
      parsed.version !== COLUMN_DRAFT_STORAGE_VERSION ||
      !Array.isArray(parsed.drafts)
    ) return {};
    const drafts: Record<string, ColumnDraftState> = {};
    for (const candidate of parsed.drafts) {
      if (!isRecord(candidate)) continue;
      const target = parseTarget(candidate.target);
      if (
        !target ||
        typeof candidate.content !== 'string' ||
        !candidate.content ||
        candidate.content.length > MAX_DRAFT_CONTENT_LENGTH
      ) continue;
      const key = columnDraftKey(target);
      drafts[key] = {
        ...createColumnDraft(target),
        content: candidate.content,
        expanded: candidate.expanded === true,
      };
    }
    return drafts;
  } catch {
    return {};
  }
}

export function writeColumnDrafts(
  storage: ColumnDraftStorage,
  drafts: Record<string, ColumnDraftState>
) {
  try {
    storage.setItem(COLUMN_DRAFT_STORAGE_KEY, serializeColumnDrafts(drafts));
    return true;
  } catch {
    return false;
  }
}

export function startColumnDraftPersistence(
  store: DesktopShellStoreApi,
  storage: ColumnDraftStorage,
  lifecycleTarget: Pick<EventTarget, 'addEventListener' | 'removeEventListener'> = window
) {
  let previous = serializeColumnDrafts(store.getState().columnDraftsByKey);
  let timeoutId: ReturnType<typeof setTimeout> | null = null;
  let pending: Record<string, ColumnDraftState> | null = null;
  let suspended = false;
  const flush = () => {
    if (timeoutId !== null) clearTimeout(timeoutId);
    timeoutId = null;
    if (suspended || !pending) return;
    const next = pending;
    pending = null;
    previous = serializeColumnDrafts(next);
    writeColumnDrafts(storage, next);
  };
  const unsubscribe = store.subscribe((state) => {
    if (suspended) return;
    const serialized = serializeColumnDrafts(state.columnDraftsByKey);
    if (serialized === previous) return;
    pending = state.columnDraftsByKey;
    if (timeoutId !== null) clearTimeout(timeoutId);
    timeoutId = setTimeout(flush, DRAFT_WRITE_DELAY_MS);
  });
  lifecycleTarget.addEventListener('pagehide', flush);
  const suspend = () => {
    if (!writeColumnDrafts(storage, store.getState().columnDraftsByKey)) throw new Error('Could not retain account drafts');
    suspended = true;
    if (timeoutId !== null) clearTimeout(timeoutId);
    timeoutId = null; pending = null;
    return () => {
      suspended = false;
      pending = store.getState().columnDraftsByKey;
      flush();
    };
  };
  const writers = activeDraftWriters.get(storage) ?? new Set();
  writers.add(suspend); activeDraftWriters.set(storage, writers);
  return () => {
    flush();
    unsubscribe();
    lifecycleTarget.removeEventListener('pagehide', flush);
    writers.delete(suspend);
  };
}
