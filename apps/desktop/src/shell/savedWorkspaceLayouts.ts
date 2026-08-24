import type { ColumnState, WorkspaceState } from '@/shell/slices/workspace';
import {
  captureWorkspaceLayoutSnapshot,
  normalizeWorkspaceLayoutSnapshot,
  type WorkspaceLayoutSnapshot,
} from '@/shell/workspacePersistence';

export const SAVED_WORKSPACE_LAYOUTS_STORAGE_KEY = 'kukuri:workspace-layouts:v1';
const SAVED_WORKSPACE_LAYOUTS_STORAGE_VERSION = 1;
const MAX_LAYOUT_NAME_LENGTH = 80;
const MAX_LAYOUT_ID_LENGTH = 128;

export type SavedWorkspaceLayoutStorage = Pick<Storage, 'getItem' | 'setItem'>;

export type SavedWorkspaceLayout = WorkspaceLayoutSnapshot & {
  id: string;
  name: string;
};

export type SavedWorkspaceLayoutNameError = 'empty' | 'duplicate' | 'too_long';

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function normalizedName(name: string) {
  return name.trim();
}

function cloneColumns(columns: readonly ColumnState[]): ColumnState[] {
  return columns.map((column) => ({
    ...column,
    ...(column.scope ? { scope: { ...column.scope } } : {}),
  }));
}

function serializeSavedWorkspaceLayouts(layouts: readonly SavedWorkspaceLayout[]) {
  return JSON.stringify({
    version: SAVED_WORKSPACE_LAYOUTS_STORAGE_VERSION,
    layouts: layouts.map((layout) => ({
      id: layout.id,
      name: layout.name,
      activeColumnId: layout.activeColumnId,
      columns: cloneColumns(layout.columns),
    })),
  });
}

export function readSavedWorkspaceLayouts(
  storage: SavedWorkspaceLayoutStorage
): SavedWorkspaceLayout[] {
  try {
    const raw = storage.getItem(SAVED_WORKSPACE_LAYOUTS_STORAGE_KEY);
    if (!raw) return [];
    const parsed: unknown = JSON.parse(raw);
    if (
      !isRecord(parsed) ||
      parsed.version !== SAVED_WORKSPACE_LAYOUTS_STORAGE_VERSION ||
      !Array.isArray(parsed.layouts)
    ) return [];
    const ids = new Set<string>();
    const names = new Set<string>();
    const layouts: SavedWorkspaceLayout[] = [];
    for (const candidate of parsed.layouts) {
      if (!isRecord(candidate)) continue;
      const id = typeof candidate.id === 'string' ? candidate.id.trim() : '';
      const name = typeof candidate.name === 'string' ? normalizedName(candidate.name) : '';
      if (
        !id ||
        id.length > MAX_LAYOUT_ID_LENGTH ||
        !name ||
        name.length > MAX_LAYOUT_NAME_LENGTH ||
        ids.has(id) ||
        names.has(name.toLowerCase())
      ) continue;
      const snapshot = normalizeWorkspaceLayoutSnapshot(candidate);
      if (!snapshot) continue;
      ids.add(id);
      names.add(name.toLowerCase());
      layouts.push({ id, name, ...snapshot });
    }
    return layouts;
  } catch {
    return [];
  }
}

export function writeSavedWorkspaceLayouts(
  storage: SavedWorkspaceLayoutStorage,
  layouts: readonly SavedWorkspaceLayout[]
) {
  try {
    storage.setItem(
      SAVED_WORKSPACE_LAYOUTS_STORAGE_KEY,
      serializeSavedWorkspaceLayouts(layouts)
    );
    return true;
  } catch {
    return false;
  }
}

export function captureSavedWorkspaceLayout(
  id: string,
  name: string,
  workspace: Pick<WorkspaceState, 'activeColumnId' | 'columns'>
): SavedWorkspaceLayout {
  return {
    id: id.trim(),
    name: normalizedName(name),
    ...captureWorkspaceLayoutSnapshot(workspace),
  };
}

export function updateSavedWorkspaceLayout(
  layout: SavedWorkspaceLayout,
  workspace: Pick<WorkspaceState, 'activeColumnId' | 'columns'>
): SavedWorkspaceLayout {
  return captureSavedWorkspaceLayout(layout.id, layout.name, workspace);
}

export function renameSavedWorkspaceLayout(
  layouts: readonly SavedWorkspaceLayout[],
  id: string,
  name: string
): SavedWorkspaceLayout[] {
  const normalized = normalizedName(name);
  return layouts.map((layout) =>
    layout.id === id ? { ...layout, name: normalized } : layout
  );
}

export function deleteSavedWorkspaceLayout(
  layouts: readonly SavedWorkspaceLayout[],
  id: string
): SavedWorkspaceLayout[] {
  return layouts.filter((layout) => layout.id !== id);
}

export function applySavedWorkspaceLayout(
  _current: WorkspaceState,
  layout: SavedWorkspaceLayout
): WorkspaceState {
  return {
    columns: cloneColumns(layout.columns),
    activeColumnId: layout.activeColumnId,
    controlCenterOpen: false,
    activeLayoutId: layout.id,
  };
}

export function isSavedWorkspaceLayoutDirty(
  workspace: Pick<WorkspaceState, 'columns'>,
  layout: SavedWorkspaceLayout
) {
  const current = captureWorkspaceLayoutSnapshot({
    columns: workspace.columns,
    activeColumnId: layout.activeColumnId,
  });
  return JSON.stringify(current.columns) !== JSON.stringify(layout.columns);
}

export function savedWorkspaceLayoutNameError(
  layouts: readonly SavedWorkspaceLayout[],
  name: string,
  excludeId?: string
): SavedWorkspaceLayoutNameError | null {
  const normalized = normalizedName(name);
  if (!normalized) return 'empty';
  if (normalized.length > MAX_LAYOUT_NAME_LENGTH) return 'too_long';
  if (
    layouts.some(
      (layout) =>
        layout.id !== excludeId && layout.name.toLowerCase() === normalized.toLowerCase()
    )
  ) return 'duplicate';
  return null;
}
