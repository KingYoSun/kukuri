import { STARTER_TOPICS } from '@/shell/slices/shared';

export type ColumnSpan = 1 | 2 | 3 | 4;

export type ColumnKind =
  | 'timeline'
  | 'notifications'
  | 'thread'
  | 'profile'
  | 'explore'
  | 'messages'
  | 'conversation'
  | 'stream'
  | 'game'
  | 'metaverse';

export type ColumnScope = {
  topicId: string;
  channelId: string | null;
};

export type ColumnState = {
  id: string;
  kind: ColumnKind;
  scope?: ColumnScope;
  entityId?: string;
  parentColumnId?: string;
  pinned: boolean;
  preferredDesktopSpan: ColumnSpan;
};

export type WorkspaceState = {
  columns: ColumnState[];
  activeColumnId: string;
  controlCenterOpen: boolean;
  activeLayoutId: string | null;
};

export type WorkspaceSliceState = {
  workspaceState: WorkspaceState;
};

export const INITIAL_TIMELINE_COLUMN_ID = 'timeline-initial';

export function createInitialWorkspaceState(
  scope: ColumnScope = { topicId: STARTER_TOPICS[0], channelId: null }
): WorkspaceState {
  return {
    columns: [
      {
        id: INITIAL_TIMELINE_COLUMN_ID,
        kind: 'timeline',
        scope,
        pinned: true,
        preferredDesktopSpan: 1,
      },
    ],
    activeColumnId: INITIAL_TIMELINE_COLUMN_ID,
    controlCenterOpen: false,
    activeLayoutId: null,
  };
}

export function createInitialWorkspaceSlice(scope?: ColumnScope): WorkspaceSliceState {
  return { workspaceState: createInitialWorkspaceState(scope) };
}

export function activateColumn(state: WorkspaceState, columnId: string): WorkspaceState {
  if (
    state.activeColumnId === columnId ||
    !state.columns.some((column) => column.id === columnId)
  ) {
    return state;
  }
  return { ...state, activeColumnId: columnId };
}

export function openTransientColumn(
  state: WorkspaceState,
  requestedColumn: ColumnState
): WorkspaceState {
  const column = { ...requestedColumn, pinned: false };
  const existingIndex = state.columns.findIndex((candidate) => candidate.id === column.id);
  if (existingIndex >= 0) {
    return activateColumn(state, column.id);
  }

  const replaceIndex = state.columns.findIndex(
    (candidate) =>
      !candidate.pinned &&
      candidate.parentColumnId === column.parentColumnId
  );
  if (replaceIndex < 0) {
    return {
      ...state,
      columns: [...state.columns, column],
      activeColumnId: column.id,
    };
  }

  const replacedId = state.columns[replaceIndex].id;
  return {
    ...state,
    columns: state.columns.map((candidate, index) => {
      if (index === replaceIndex) return column;
      if (candidate.parentColumnId === replacedId) {
        return { ...candidate, parentColumnId: column.id };
      }
      return candidate;
    }),
    activeColumnId: column.id,
  };
}

export function setColumnPinned(
  state: WorkspaceState,
  columnId: string,
  pinned: boolean
): WorkspaceState {
  let changed = false;
  const columns = state.columns.map((column) => {
    if (column.id !== columnId || column.pinned === pinned) return column;
    changed = true;
    return { ...column, pinned };
  });
  return changed ? { ...state, columns } : state;
}

export function closeColumn(state: WorkspaceState, columnId: string): WorkspaceState {
  if (state.columns.length <= 1) return state;

  const closingIndex = state.columns.findIndex((column) => column.id === columnId);
  if (closingIndex < 0) return state;

  const closingColumn = state.columns[closingIndex];
  const columns = state.columns
    .filter((column) => column.id !== columnId)
    .map((column) =>
      column.parentColumnId === columnId
        ? { ...column, parentColumnId: closingColumn.parentColumnId }
        : column
    );
  if (state.activeColumnId !== columnId) return { ...state, columns };

  const parent = closingColumn.parentColumnId
    ? columns.find((column) => column.id === closingColumn.parentColumnId)
    : undefined;
  const neighbor = columns[Math.min(closingIndex, columns.length - 1)];
  return {
    ...state,
    columns,
    activeColumnId: parent?.id ?? neighbor.id,
  };
}

export function updateColumnScope(
  state: WorkspaceState,
  columnId: string,
  scope: ColumnScope
): WorkspaceState {
  let changed = false;
  const columns = state.columns.map((column) => {
    if (column.id !== columnId) return column;
    if (
      column.scope?.topicId === scope.topicId &&
      column.scope.channelId === scope.channelId
    ) {
      return column;
    }
    changed = true;
    return { ...column, scope };
  });
  return changed ? { ...state, columns } : state;
}
