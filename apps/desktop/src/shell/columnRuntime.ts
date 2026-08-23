import type { ColumnKind } from '@/shell/slices/workspace';

export type ColumnRuntimeState = {
  visible: boolean;
  active: boolean;
  audioFocused: boolean;
  suspended: boolean;
};

type ColumnRuntimeProjectionInput = {
  kind: ColumnKind;
  columnId: string;
  activeColumnId: string;
  visibleColumnIds: ReadonlySet<string>;
  audioFocusedColumnId: string | null;
};

export function projectColumnRuntime({
  kind,
  columnId,
  activeColumnId,
  visibleColumnIds,
  audioFocusedColumnId,
}: ColumnRuntimeProjectionInput): ColumnRuntimeState {
  const visible = visibleColumnIds.has(columnId);
  const audioFocused = audioFocusedColumnId === columnId;
  const immersive = kind === 'stream' || kind === 'metaverse';
  return {
    visible,
    active: activeColumnId === columnId,
    audioFocused,
    suspended: immersive && !visible && !audioFocused,
  };
}

export function requestColumnAudioFocus(_current: string | null, columnId: string) {
  return columnId;
}

export function releaseColumnAudioFocus(current: string | null, columnId: string) {
  return current === columnId ? null : current;
}
