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

export type ColumnStateInput = Omit<ColumnState, 'preferredDesktopSpan'> & {
  preferredDesktopSpan?: number;
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

type ColumnSpanPolicy = {
  default: ColumnSpan;
  min: ColumnSpan;
  max: ColumnSpan;
};

const FIXED_COLUMN_SPAN_POLICY: ColumnSpanPolicy = { default: 1, min: 1, max: 1 };

const COLUMN_SPAN_POLICIES: Record<ColumnKind, ColumnSpanPolicy> = {
  timeline: FIXED_COLUMN_SPAN_POLICY,
  notifications: FIXED_COLUMN_SPAN_POLICY,
  thread: FIXED_COLUMN_SPAN_POLICY,
  profile: FIXED_COLUMN_SPAN_POLICY,
  explore: FIXED_COLUMN_SPAN_POLICY,
  messages: { default: 1, min: 1, max: 2 },
  conversation: { default: 1, min: 1, max: 2 },
  stream: { default: 2, min: 1, max: 2 },
  game: FIXED_COLUMN_SPAN_POLICY,
  metaverse: { default: 3, min: 1, max: 4 },
};

export function columnSpanPolicy(kind: ColumnKind): ColumnSpanPolicy {
  return COLUMN_SPAN_POLICIES[kind];
}

export function defaultColumnSpan(kind: ColumnKind): ColumnSpan {
  return columnSpanPolicy(kind).default;
}

export function normalizeColumnSpan(kind: ColumnKind, requestedSpan?: number): ColumnSpan {
  const policy = columnSpanPolicy(kind);
  if (!Number.isFinite(requestedSpan)) return policy.default;
  return Math.min(policy.max, Math.max(policy.min, Math.round(requestedSpan!))) as ColumnSpan;
}

function resolveColumnInput(
  requestedColumn: ColumnStateInput,
  fallbackSpan?: ColumnSpan
): ColumnState {
  return {
    ...requestedColumn,
    preferredDesktopSpan: normalizeColumnSpan(
      requestedColumn.kind,
      requestedColumn.preferredDesktopSpan ?? fallbackSpan
    ),
  };
}

function identityPart(value: string | null | undefined) {
  return value ? encodeURIComponent(value) : '-';
}

export function columnIdentityId(
  kind: ColumnKind,
  scope?: ColumnScope,
  entityId?: string
): string {
  return [
    'column',
    kind,
    identityPart(scope?.topicId),
    identityPart(scope?.channelId),
    identityPart(entityId),
  ].join(':');
}

export const INITIAL_TIMELINE_COLUMN_ID = columnIdentityId('timeline', {
  topicId: STARTER_TOPICS[0],
  channelId: null,
});

export function createInitialWorkspaceState(
  scope: ColumnScope = { topicId: STARTER_TOPICS[0], channelId: null }
): WorkspaceState {
  const initialTimelineColumnId = columnIdentityId('timeline', scope);
  return {
    columns: [
      {
        id: initialTimelineColumnId,
        kind: 'timeline',
        scope,
        pinned: true,
        preferredDesktopSpan: defaultColumnSpan('timeline'),
      },
    ],
    activeColumnId: initialTimelineColumnId,
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
  requestedColumn: ColumnStateInput
): WorkspaceState {
  const existingIndex = state.columns.findIndex((candidate) => candidate.id === requestedColumn.id);
  const column = {
    ...resolveColumnInput(requestedColumn, state.columns[existingIndex]?.preferredDesktopSpan),
    pinned: false,
  };
  if (existingIndex >= 0) {
    const existing = state.columns[existingIndex];
    if (existing.pinned) return activateColumn(state, column.id);

    const columns = state.columns.filter((candidate) => candidate.id !== column.id);
    const parentIndex = column.parentColumnId
      ? columns.findIndex((candidate) => candidate.id === column.parentColumnId)
      : -1;
    const insertIndex = parentIndex >= 0 ? parentIndex + 1 : columns.length;
    columns.splice(insertIndex, 0, column);
    return {
      ...state,
      columns,
      activeColumnId: column.id,
    };
  }

  const replaceIndex = state.columns.findIndex(
    (candidate) =>
      !candidate.pinned &&
      candidate.parentColumnId === column.parentColumnId
  );
  if (replaceIndex < 0) {
    const parentIndex = column.parentColumnId
      ? state.columns.findIndex((candidate) => candidate.id === column.parentColumnId)
      : -1;
    const insertIndex = parentIndex >= 0 ? parentIndex + 1 : state.columns.length;
    const columns = [...state.columns];
    columns.splice(insertIndex, 0, column);
    return {
      ...state,
      columns,
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

export function openPinnedColumn(
  state: WorkspaceState,
  requestedColumn: ColumnStateInput
): WorkspaceState {
  const existing = state.columns.find((column) => column.id === requestedColumn.id);
  if (existing) {
    const pinned = setColumnPinned(state, existing.id, true);
    return activateColumn(pinned, existing.id);
  }

  const column = { ...resolveColumnInput(requestedColumn), pinned: true };
  return {
    ...state,
    columns: [...state.columns, column],
    activeColumnId: column.id,
  };
}

export function setColumnSpan(
  state: WorkspaceState,
  columnId: string,
  requestedSpan: number
): WorkspaceState {
  let changed = false;
  const columns = state.columns.map((column) => {
    if (column.id !== columnId) return column;
    const preferredDesktopSpan = normalizeColumnSpan(column.kind, requestedSpan);
    if (preferredDesktopSpan === column.preferredDesktopSpan) return column;
    changed = true;
    return { ...column, preferredDesktopSpan };
  });
  return changed ? { ...state, columns } : state;
}

export function moveColumn(
  state: WorkspaceState,
  columnId: string,
  requestedIndex: number
): WorkspaceState {
  const currentIndex = state.columns.findIndex((column) => column.id === columnId);
  if (currentIndex < 0 || !Number.isFinite(requestedIndex)) return state;
  const targetIndex = Math.min(
    state.columns.length - 1,
    Math.max(0, Math.round(requestedIndex))
  );
  if (targetIndex === currentIndex) return state;
  const columns = [...state.columns];
  const [column] = columns.splice(currentIndex, 1);
  columns.splice(targetIndex, 0, column);
  return { ...state, columns };
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
