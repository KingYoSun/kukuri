import { useEffect, type ReactNode } from 'react';

import { ColumnCanvas } from '@/components/shell/ColumnCanvas';
import { ColumnSurface } from '@/components/shell/ColumnSurface';
import {
  activateColumn,
  INITIAL_TIMELINE_COLUMN_ID,
  setColumnPinned,
  updateColumnScope,
} from '@/shell/slices/workspace';
import { useDesktopShellFieldSetter, useDesktopShellStore } from '@/shell/store';

type DesktopShellColumnWorkspaceProps = {
  children: ReactNode;
  scopeLabel: string;
  title: string;
};

export function DesktopShellColumnWorkspace({
  children,
  scopeLabel,
  title,
}: DesktopShellColumnWorkspaceProps) {
  const activeTopic = useDesktopShellStore((state) => state.activeTopic);
  const selectedChannelId = useDesktopShellStore(
    (state) => state.selectedChannelIdByTopic[state.activeTopic] ?? null
  );
  const workspaceState = useDesktopShellStore((state) => state.workspaceState);
  const setWorkspaceState = useDesktopShellFieldSetter('workspaceState');

  useEffect(() => {
    setWorkspaceState((current) =>
      updateColumnScope(current, INITIAL_TIMELINE_COLUMN_ID, {
        topicId: activeTopic,
        channelId: selectedChannelId,
      })
    );
  }, [activeTopic, selectedChannelId, setWorkspaceState]);

  const timelineColumn = workspaceState.columns.find(
    (column) => column.id === INITIAL_TIMELINE_COLUMN_ID
  ) ?? workspaceState.columns[0];

  return (
    <ColumnCanvas
      activeColumnId={workspaceState.activeColumnId}
      onActivateColumn={(columnId) =>
        setWorkspaceState((current) => activateColumn(current, columnId))
      }
    >
      <ColumnSurface
        columnId={timelineColumn.id}
        title={title}
        scopeLabel={`${scopeLabel} · ${activeTopic}`}
        position={1}
        total={1}
        span={1}
        active={workspaceState.activeColumnId === timelineColumn.id}
        pinned={timelineColumn.pinned}
        onPinnedChange={(pinned) =>
          setWorkspaceState((current) => setColumnPinned(current, timelineColumn.id, pinned))
        }
      >
        {children}
      </ColumnSurface>
    </ColumnCanvas>
  );
}
