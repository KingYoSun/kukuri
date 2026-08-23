import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { columnDraftKey, createColumnDraft, type ColumnDraftTarget } from '@/shell/slices/columnDrafts';
import { createDesktopShellStore } from '@/shell/store';
import {
  readColumnDrafts,
  startColumnDraftPersistence,
  writeColumnDrafts,
  type ColumnDraftStorage,
} from '@/shell/columnDraftPersistence';

const target: ColumnDraftTarget = {
  columnId: 'timeline-public',
  action: 'post',
  scope: { topicId: 'topic-a', channelId: null },
};

function memoryStorage(initial: string | null = null): ColumnDraftStorage & {
  getItem: ReturnType<typeof vi.fn>;
  setItem: ReturnType<typeof vi.fn>;
} {
  let value = initial;
  return {
    getItem: vi.fn(() => value),
    setItem: vi.fn((_key: string, next: string) => {
      value = next;
    }),
  };
}

describe('Column Draft persistence', () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it('round-trips only serializable text Draft state', () => {
    const storage = memoryStorage();
    const draft = {
      ...createColumnDraft(target),
      content: 'restart me',
      expanded: true,
      pending: true,
      error: 'not persisted',
      attachmentInputKey: 4,
      mediaItems: [{ id: 'image', source_name: 'image.png', preview_url: 'blob:image', attachments: [] }],
    };

    expect(writeColumnDrafts(storage, { [columnDraftKey(target)]: draft })).toBe(true);
    const raw = storage.setItem.mock.calls[0]?.[1] ?? '';
    expect(raw).not.toContain('blob:image');
    expect(raw).not.toContain('not persisted');
    expect(readColumnDrafts(storage)[columnDraftKey(target)]).toEqual({
      ...createColumnDraft(target),
      content: 'restart me',
      expanded: true,
    });
  });

  it('drops empty, malformed, and unknown-version Draft entries safely', () => {
    const emptyStorage = memoryStorage();
    expect(writeColumnDrafts(emptyStorage, { [columnDraftKey(target)]: createColumnDraft(target) })).toBe(true);
    expect(emptyStorage.setItem.mock.calls[0]?.[1]).not.toContain('timeline-public');

    expect(readColumnDrafts(memoryStorage('{'))).toEqual({});
    expect(readColumnDrafts(memoryStorage(JSON.stringify({ version: 2, drafts: [] })))).toEqual({});
    expect(readColumnDrafts(memoryStorage(JSON.stringify({
      version: 1,
      drafts: [
        { target: { columnId: '', action: 'post' }, content: 'bad' },
        { target, content: '', expanded: false },
      ],
    })))).toEqual({});
  });

  it('restores at store creation and debounces writes with pagehide flush and cleanup', () => {
    const storage = memoryStorage();
    writeColumnDrafts(storage, {
      [columnDraftKey(target)]: { ...createColumnDraft(target), content: 'saved' },
    });
    storage.setItem.mockClear();
    const events = new EventTarget();
    const store = createDesktopShellStore({ draftStorage: storage });
    expect(store.getState().columnDraftsByKey[columnDraftKey(target)]?.content).toBe('saved');

    const stop = startColumnDraftPersistence(store, storage, events);
    store.getState().setField('columnDraftsByKey', {
      [columnDraftKey(target)]: { ...createColumnDraft(target), content: 'next' },
    });
    expect(storage.setItem).not.toHaveBeenCalled();
    events.dispatchEvent(new Event('pagehide'));
    expect(storage.setItem).toHaveBeenCalledTimes(1);

    store.getState().setField('columnDraftsByKey', {
      [columnDraftKey(target)]: { ...createColumnDraft(target), content: 'after' },
    });
    stop();
    expect(storage.setItem).toHaveBeenCalledTimes(2);
    vi.runAllTimers();
    expect(storage.setItem).toHaveBeenCalledTimes(2);
  });

  it('keeps storage failures non-fatal', () => {
    const broken: ColumnDraftStorage = {
      getItem: () => { throw new Error('denied'); },
      setItem: () => { throw new Error('quota'); },
    };
    expect(readColumnDrafts(broken)).toEqual({});
    expect(writeColumnDrafts(broken, {})).toBe(false);
  });
});
