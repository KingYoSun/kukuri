import { useDesktopShellFieldSetter, useDesktopShellStore } from '@/shell/store';
import { activateColumn, type ColumnState } from '@/shell/slices/workspace';
import type { useShellDialogs } from '@/shell/page/useShellDialogs';
import type { useDesktopShellActions } from '@/shell/useDesktopShellActions';

type UsePrivateChannelEntriesArgs = {
  dialogs: Pick<
    ReturnType<typeof useShellDialogs>,
    'setChannelDialogOpen' | 'setChannelSettingsDialogOpen'
  >;
  shellActions: Pick<
    ReturnType<typeof useDesktopShellActions>,
    'handleSelectPrivateChannel' | 'handleSelectTopic'
  >;
  activateWorkspaceColumn: (column: ColumnState) => Promise<void>;
};

// Issue #966: 通常画面からプライベートチャンネルの作成・参加・設定・共有 Dialog へ入る共通経路。
// Control Center、Timeline Column header、作成・参加 Dialog の参加済み一覧が同じ helper を使う。
// Dialog を開くだけで API は呼ばない(作成・参加・共有は Dialog 内の既存 handler が担う)。
export function usePrivateChannelEntries({
  dialogs,
  shellActions,
  activateWorkspaceColumn,
}: UsePrivateChannelEntriesArgs) {
  const { setChannelDialogOpen, setChannelSettingsDialogOpen } = dialogs;
  const { handleSelectPrivateChannel, handleSelectTopic } = shellActions;
  const activeColumnId = useDesktopShellStore((state) => state.workspaceState.activeColumnId);
  const setWorkspaceState = useDesktopShellFieldSetter('workspaceState');
  const setInviteOutput = useDesktopShellFieldSetter('inviteOutput');
  const setChannelError = useDesktopShellFieldSetter('channelError');

  // 非 active Column の header から開いた場合は、その Column を先に active にして
  // Dialog の対象 topic を Column の scope に揃える。
  const activateColumnIfNeeded = (column: ColumnState) => {
    if (activeColumnId === column.id) return;
    setWorkspaceState((current) => activateColumn(current, column.id));
    void activateWorkspaceColumn(column);
  };

  const openChannelSettingsDialog = (topic: string, channelId: string) => {
    setInviteOutput(null);
    setChannelError(null);
    handleSelectPrivateChannel(topic, channelId);
    setChannelSettingsDialogOpen(true);
  };

  return {
    openChannelSettingsDialog,
    openChannelManagerForColumn: (column: ColumnState) => {
      activateColumnIfNeeded(column);
      setChannelDialogOpen(true);
    },
    openChannelSettingsForColumn: (column: ColumnState, topic: string, channelId: string) => {
      activateColumnIfNeeded(column);
      openChannelSettingsDialog(topic, channelId);
    },
    openChannelManagerForTopic: (topic: string) => {
      void handleSelectTopic(topic);
      setChannelDialogOpen(true);
    },
  };
}
