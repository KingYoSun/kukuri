import { describe, expect, it, vi } from 'vitest';

import {
  applySavedWorkspaceLayout,
  captureSavedWorkspaceLayout,
  deleteSavedWorkspaceLayout,
  isSavedWorkspaceLayoutDirty,
  readSavedWorkspaceLayouts,
  renameSavedWorkspaceLayout,
  SAVED_WORKSPACE_LAYOUTS_STORAGE_KEY,
  savedWorkspaceLayoutNameError,
  updateSavedWorkspaceLayout,
  writeSavedWorkspaceLayouts,
  type SavedWorkspaceLayoutStorage,
} from '@/shell/savedWorkspaceLayouts';
import { createInitialWorkspaceState } from '@/shell/slices/workspace';

function memoryStorage(initial: string | null = null): SavedWorkspaceLayoutStorage & {
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

describe('saved workspace layouts', () => {
  it('round-trips named snapshots without transient or draft state', () => {
    const workspace = createInitialWorkspaceState();
    const openWorkspace = {
      ...workspace,
      controlCenterOpen: true,
      activeLayoutId: 'other-layout',
    };
    const saved = captureSavedWorkspaceLayout('layout-1', 'Research', openWorkspace);
    const storage = memoryStorage();

    expect(writeSavedWorkspaceLayouts(storage, [saved])).toBe(true);
    expect(storage.setItem).toHaveBeenCalledWith(
      SAVED_WORKSPACE_LAYOUTS_STORAGE_KEY,
      expect.any(String)
    );
    expect(readSavedWorkspaceLayouts(storage)).toEqual([saved]);
    const payload = JSON.parse(storage.setItem.mock.calls[0][1] as string);
    expect(payload).not.toHaveProperty('controlCenterOpen');
    expect(JSON.stringify(payload)).not.toContain('draft');
  });

  it('normalizes invalid columns, parents, spans, active ids, duplicate layout ids and names', () => {
    const storage = memoryStorage(JSON.stringify({
      version: 1,
      layouts: [
        {
          id: 'layout-1',
          name: 'Research',
          activeColumnId: 'missing',
          columns: [
            { id: 'stream', kind: 'stream', pinned: true, preferredDesktopSpan: 99 },
            { id: 'profile', kind: 'profile', pinned: false, preferredDesktopSpan: 4, parentColumnId: 'missing' },
          ],
        },
        {
          id: 'layout-1',
          name: 'Duplicate id',
          activeColumnId: 'timeline',
          columns: [{ id: 'timeline', kind: 'timeline', pinned: true, preferredDesktopSpan: 1 }],
        },
        {
          id: 'layout-2',
          name: ' research ',
          activeColumnId: 'timeline',
          columns: [{ id: 'timeline', kind: 'timeline', pinned: true, preferredDesktopSpan: 1 }],
        },
      ],
    }));

    expect(readSavedWorkspaceLayouts(storage)).toEqual([
      {
        id: 'layout-1',
        name: 'Research',
        activeColumnId: 'stream',
        columns: [
          { id: 'stream', kind: 'stream', pinned: true, preferredDesktopSpan: 2 },
          { id: 'profile', kind: 'profile', pinned: false, preferredDesktopSpan: 1 },
        ],
      },
    ]);
  });

  it('rejects malformed or unknown payloads and tolerates storage failures', () => {
    expect(readSavedWorkspaceLayouts(memoryStorage('{'))).toEqual([]);
    expect(readSavedWorkspaceLayouts(memoryStorage(JSON.stringify({ version: 2, layouts: [] })))).toEqual([]);
    expect(readSavedWorkspaceLayouts({
      getItem: () => { throw new Error('denied'); },
      setItem: vi.fn(),
    })).toEqual([]);
    expect(writeSavedWorkspaceLayouts({
      getItem: vi.fn(),
      setItem: () => { throw new Error('denied'); },
    }, [])).toBe(false);
  });

  it('captures, applies, updates, renames and deletes layouts with active id semantics', () => {
    const initial = createInitialWorkspaceState();
    const saved = captureSavedWorkspaceLayout('layout-1', 'Research', initial);
    const changed = {
      ...initial,
      columns: initial.columns.map((column) => ({ ...column, pinned: false })),
      controlCenterOpen: true,
    };

    expect(isSavedWorkspaceLayoutDirty(changed, saved)).toBe(true);
    const focusChanged = { ...initial, activeColumnId: 'missing' };
    expect(isSavedWorkspaceLayoutDirty(focusChanged, saved)).toBe(false);

    const applied = applySavedWorkspaceLayout(changed, saved);
    expect(applied).toEqual({
      columns: saved.columns,
      activeColumnId: saved.activeColumnId,
      controlCenterOpen: false,
      activeLayoutId: saved.id,
    });

    const updated = updateSavedWorkspaceLayout(saved, changed);
    expect(updated.name).toBe('Research');
    expect(updated.columns[0].pinned).toBe(false);
    expect(renameSavedWorkspaceLayout([updated], saved.id, 'Reading')[0].name).toBe('Reading');
    expect(deleteSavedWorkspaceLayout([updated], saved.id)).toEqual([]);
  });

  it('validates empty and duplicate names case-insensitively', () => {
    const layout = captureSavedWorkspaceLayout('layout-1', 'Research', createInitialWorkspaceState());
    expect(savedWorkspaceLayoutNameError([layout], '   ')).toBe('empty');
    expect(savedWorkspaceLayoutNameError([layout], ' research ')).toBe('duplicate');
    expect(savedWorkspaceLayoutNameError([layout], ' research ', layout.id)).toBeNull();
    expect(savedWorkspaceLayoutNameError([layout], 'Reading')).toBeNull();
  });
});
