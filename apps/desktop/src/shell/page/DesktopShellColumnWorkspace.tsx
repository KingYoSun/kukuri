import { useEffect, type ReactNode } from 'react';

import { ColumnCanvas } from '@/components/shell/ColumnCanvas';
import { ColumnSurface } from '@/components/shell/ColumnSurface';
import {
  TimelineViewIconTabs,
  type TimelineViewId,
} from '@/components/shell/TimelineViewIconTabs';
import type { PrimarySection } from '@/components/shell/types';
import {
  activateColumn,
  closeColumn,
  INITIAL_TIMELINE_COLUMN_ID,
  setColumnPinned,
  updateColumnScope,
  type ColumnKind,
  type ColumnState,
} from '@/shell/slices/workspace';
import { useDesktopShellFieldSetter, useDesktopShellStore } from '@/shell/store';

type DesktopShellColumnWorkspaceProps = {
  activeTimelineView: TimelineViewId;
  renderConversationSurface: (column: ColumnState) => ReactNode;
  messagesSurface: ReactNode;
  notificationsSurface: ReactNode;
  onActivateColumn: (column: ColumnState) => void;
  onSelectTimelineView: (view: TimelineViewId) => void;
  renderProfileSurface: (column: ColumnState) => ReactNode;
  renderPrimarySurface: (section: PrimarySection, column: ColumnState) => ReactNode;
  scopeLabel: string;
  renderThreadSurface: (column: ColumnState) => ReactNode;
  timelineViewItems: Array<{ id: TimelineViewId; label: string }>;
  titles: Record<ColumnKind, string>;
};

const PRIMARY_SECTION_BY_KIND: Partial<Record<ColumnKind, PrimarySection>> = {
  timeline: 'timeline',
  notifications: 'notifications',
  explore: 'explore',
  messages: 'messages',
  profile: 'profile',
  stream: 'live',
  game: 'game',
  metaverse: 'game',
};

export function DesktopShellColumnWorkspace({
  activeTimelineView,
  renderConversationSurface,
  messagesSurface,
  notificationsSurface,
  onActivateColumn,
  onSelectTimelineView,
  renderProfileSurface,
  renderPrimarySurface,
  scopeLabel,
  renderThreadSurface,
  timelineViewItems,
  titles,
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

  const activate = (columnId: string, syncRoute: boolean) => {
    const column = workspaceState.columns.find((candidate) => candidate.id === columnId);
    if (!column) return;
    setWorkspaceState((current) => activateColumn(current, columnId));
    if (syncRoute) onActivateColumn(column);
  };
  const close = (columnId: string) => {
    const next = closeColumn(workspaceState, columnId);
    if (next === workspaceState) return;
    setWorkspaceState(next);
    const activeColumn = next.columns.find((column) => column.id === next.activeColumnId);
    if (activeColumn) onActivateColumn(activeColumn);
  };
  const renderBody = (column: ColumnState) => {
    if (column.kind === 'thread') return renderThreadSurface(column);
    if (column.kind === 'profile' && column.entityId) return renderProfileSurface(column);
    if (column.kind === 'conversation') return renderConversationSurface(column);
    if (column.kind === 'messages') return messagesSurface;
    if (column.kind === 'notifications') return notificationsSurface;
    const primarySection = PRIMARY_SECTION_BY_KIND[column.kind];
    return primarySection ? renderPrimarySurface(primarySection, column) : null;
  };
  const columnScopeLabel = (column: ColumnState) => {
    if (column.entityId) return column.entityId;
    if (!column.scope) return scopeLabel;
    const channel = column.scope.channelId ?? 'Public';
    return `${channel} · ${column.scope.topicId}`;
  };

  return (
    <ColumnCanvas activeColumnId={workspaceState.activeColumnId} onActivateColumn={activate}>
      {workspaceState.columns.map((column, index) => (
        <ColumnSurface
          key={column.id}
          columnId={column.id}
          title={titles[column.kind]}
          scopeLabel={columnScopeLabel(column)}
          position={index + 1}
          total={workspaceState.columns.length}
          span={column.preferredDesktopSpan}
          active={workspaceState.activeColumnId === column.id}
          pinned={column.pinned}
          headerActions={
            column.kind === 'timeline' ? (
              <TimelineViewIconTabs
                activeView={activeTimelineView}
                items={timelineViewItems}
                onSelect={onSelectTimelineView}
              />
            ) : undefined
          }
          onPinnedChange={(pinned) =>
            setWorkspaceState((current) => setColumnPinned(current, column.id, pinned))
          }
          onClose={workspaceState.columns.length > 1 ? () => close(column.id) : undefined}
        >
          {renderBody(column)}
        </ColumnSurface>
      ))}
    </ColumnCanvas>
  );
}
