import {
  useCallback,
  useEffect,
  useMemo,
  useState,
  type ChangeEvent,
  type FormEvent,
  type ReactNode,
} from 'react';

import { ColumnComposerFooter } from '@/components/shell/ColumnComposerFooter';
import { ColumnDomainActionFooter } from '@/components/shell/ColumnDomainActionFooter';
import { ColumnCanvas } from '@/components/shell/ColumnCanvas';
import { ColumnSurface } from '@/components/shell/ColumnSurface';
import {
  TimelineViewIconTabs,
  type TimelineViewId,
} from '@/components/shell/TimelineViewIconTabs';
import type { PrimarySection } from '@/components/shell/types';
import type { MentionCandidate } from '@/components/core/types';
import { authorDisplayLabel, shortPubkey } from '@/shell/presentation';
import type { ColumnDraftTarget } from '@/shell/slices/columnDrafts';
import {
  activateColumn,
  closeColumn,
  columnSpanPolicy,
  moveColumn,
  setColumnPinned,
  setColumnSpan,
  type ColumnKind,
  type ColumnState,
} from '@/shell/slices/workspace';
import { useDesktopShellFieldSetter, useDesktopShellStore } from '@/shell/store';
import { ColumnRuntimeProvider } from '@/shell/ColumnRuntimeContext';
import {
  projectColumnRuntime,
  releaseColumnAudioFocus,
  requestColumnAudioFocus,
} from '@/shell/columnRuntime';

type DesktopShellColumnWorkspaceProps = {
  activeTimelineView: TimelineViewId;
  locale: string;
  mentionCandidates: MentionCandidate[];
  onColumnAttachmentSelection: (
    target: ColumnDraftTarget,
    event: ChangeEvent<HTMLInputElement>
  ) => Promise<void>;
  onRemoveColumnAttachment: (target: ColumnDraftTarget, itemId: string) => void;
  onSubmitColumnDraft: (
    target: ColumnDraftTarget,
    event: FormEvent<HTMLFormElement>
  ) => Promise<void>;
  onEndLiveSession: (sessionId: string, topic: string) => Promise<void>;
  onJoinLiveSession: (sessionId: string, topic: string) => Promise<void>;
  onLeaveLiveSession: (sessionId: string, topic: string) => Promise<void>;
  onOpenGameCreate: () => void;
  onOpenLiveCreate: () => void;
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
  locale,
  mentionCandidates,
  onColumnAttachmentSelection,
  onRemoveColumnAttachment,
  onSubmitColumnDraft,
  onEndLiveSession,
  onJoinLiveSession,
  onLeaveLiveSession,
  onOpenGameCreate,
  onOpenLiveCreate,
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
  const workspaceState = useDesktopShellStore((state) => state.workspaceState);
  const joinedChannelsByTopic = useDesktopShellStore((state) => state.joinedChannelsByTopic);
  const directMessages = useDesktopShellStore((state) => state.directMessages);
  const knownAuthorsByPubkey = useDesktopShellStore((state) => state.knownAuthorsByPubkey);
  const setWorkspaceState = useDesktopShellFieldSetter('workspaceState');
  const [visibleColumnIds, setVisibleColumnIds] = useState<string[]>([
    workspaceState.activeColumnId,
  ]);
  const [audioFocusedColumnId, setAudioFocusedColumnId] = useState<string | null>(null);

  const updateVisibleColumnIds = useCallback((columnIds: string[]) => {
    setVisibleColumnIds((current) =>
      current.length === columnIds.length && current.every((id, index) => id === columnIds[index])
        ? current
        : columnIds
    );
  }, []);

  useEffect(() => {
    const ids = new Set(workspaceState.columns.map((column) => column.id));
    setVisibleColumnIds((current) => current.filter((id) => ids.has(id)));
    setAudioFocusedColumnId((current) => current && ids.has(current) ? current : null);
  }, [workspaceState.columns]);

  const visibleColumnIdSet = useMemo(() => new Set(visibleColumnIds), [visibleColumnIds]);

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
  const scopeDestinationLabel = (column: ColumnState) => {
    if (!column.scope) return scopeLabel;
    const topicLabel = column.scope.topicId.split(':').filter(Boolean).at(-1) ?? column.scope.topicId;
    const channel = column.scope.channelId
      ? joinedChannelsByTopic[column.scope.topicId]?.find(
          (candidate) => candidate.channel_id === column.scope?.channelId
        )?.label ?? column.scope.channelId
      : 'Public';
    return `${channel} · ${topicLabel}`;
  };
  const conversationLabel = (peerPubkey: string) => {
    const conversation = directMessages.find((item) => item.peer_pubkey === peerPubkey);
    const author = knownAuthorsByPubkey[peerPubkey];
    return authorDisplayLabel(
      peerPubkey,
      author?.display_name ?? conversation?.peer_display_name,
      author?.name ?? conversation?.peer_name
    );
  };
  const columnScopeLabel = (column: ColumnState) => {
    if (column.kind === 'conversation' && column.entityId) {
      return conversationLabel(column.entityId);
    }
    if (column.kind === 'profile' && column.entityId) {
      const author = knownAuthorsByPubkey[column.entityId];
      return authorDisplayLabel(column.entityId, author?.display_name, author?.name);
    }
    if (column.kind === 'thread') return `Thread · ${scopeDestinationLabel(column)}`;
    if (column.entityId) return shortPubkey(column.entityId);
    return scopeDestinationLabel(column);
  };
  const renderFooter = (column: ColumnState, active: boolean) => {
    const common = {
      active,
      locale,
      mentionCandidates,
      onActivate: () => activate(column.id, true),
      onAttachmentSelection: onColumnAttachmentSelection,
      onRemoveAttachment: onRemoveColumnAttachment,
      onSubmit: onSubmitColumnDraft,
    };
    if (column.kind === 'timeline' && column.scope) {
      return (
        <ColumnComposerFooter
          {...common}
          destinationLabel={scopeDestinationLabel(column)}
          target={{ columnId: column.id, action: 'post', scope: column.scope }}
        />
      );
    }
    if (column.kind === 'thread' && column.scope && column.entityId) {
      return (
        <ColumnComposerFooter
          {...common}
          destinationLabel={`Thread · ${scopeDestinationLabel(column)}`}
          target={{
            columnId: column.id,
            action: 'reply',
            scope: column.scope,
            threadId: column.entityId,
          }}
        />
      );
    }
    if (column.kind === 'conversation' && column.entityId) {
      return (
        <ColumnComposerFooter
          {...common}
          destinationLabel={conversationLabel(column.entityId)}
          target={{ columnId: column.id, action: 'message', peerPubkey: column.entityId }}
        />
      );
    }
    if (column.kind === 'stream' || column.kind === 'game' || column.kind === 'metaverse') {
      return (
        <ColumnDomainActionFooter
          active={active}
          column={column}
          onActivate={() => activate(column.id, true)}
          onEndLiveSession={onEndLiveSession}
          onJoinLiveSession={onJoinLiveSession}
          onLeaveLiveSession={onLeaveLiveSession}
          onOpenGameCreate={onOpenGameCreate}
          onOpenLiveCreate={onOpenLiveCreate}
        />
      );
    }
    return undefined;
  };

  return (
    <ColumnCanvas
      activeColumnId={workspaceState.activeColumnId}
      columnIds={workspaceState.columns.map((column) => column.id)}
      onActivateColumn={activate}
      onVisibleColumnIdsChange={updateVisibleColumnIds}
      onMoveColumn={(columnId, targetIndex) =>
        setWorkspaceState((current) => moveColumn(current, columnId, targetIndex))
      }
    >
      {workspaceState.columns.map((column, index) => {
        const runtime = projectColumnRuntime({
          kind: column.kind,
          columnId: column.id,
          activeColumnId: workspaceState.activeColumnId,
          visibleColumnIds: visibleColumnIdSet,
          audioFocusedColumnId,
        });
        return (
          <ColumnRuntimeProvider
            key={column.id}
            value={{
              ...runtime,
              requestAudioFocus: () =>
                setAudioFocusedColumnId((current) =>
                  requestColumnAudioFocus(current, column.id)
                ),
              releaseAudioFocus: () =>
                setAudioFocusedColumnId((current) =>
                  releaseColumnAudioFocus(current, column.id)
                ),
            }}
          >
            <ColumnSurface
              columnId={column.id}
              title={titles[column.kind]}
              scopeLabel={columnScopeLabel(column)}
              position={index + 1}
              total={workspaceState.columns.length}
              span={column.preferredDesktopSpan}
              spanOptions={(() => {
                const policy = columnSpanPolicy(column.kind);
                return Array.from(
                  { length: policy.max - policy.min + 1 },
                  (_, optionIndex) => (policy.min + optionIndex) as 1 | 2 | 3 | 4
                );
              })()}
              active={runtime.active}
              pinned={column.pinned}
              resourceManaged={column.kind === 'stream' || column.kind === 'metaverse'}
              footer={renderFooter(column, runtime.active)}
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
              onMoveLeft={
                index > 0
                  ? () =>
                      setWorkspaceState((current) => moveColumn(current, column.id, index - 1))
                  : undefined
              }
              onMoveRight={
                index < workspaceState.columns.length - 1
                  ? () =>
                      setWorkspaceState((current) => moveColumn(current, column.id, index + 1))
                  : undefined
              }
              onSpanChange={(span) =>
                setWorkspaceState((current) => setColumnSpan(current, column.id, span))
              }
              onClose={workspaceState.columns.length > 1 ? () => close(column.id) : undefined}
            >
              {renderBody(column)}
            </ColumnSurface>
          </ColumnRuntimeProvider>
        );
      })}
    </ColumnCanvas>
  );
}
