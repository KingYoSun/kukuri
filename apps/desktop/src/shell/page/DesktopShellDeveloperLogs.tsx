import { DeveloperLogViewer } from '@/components/settings/DeveloperLogViewer';
import type { DesktopApi } from '@/lib/api';
import { useDesktopLogs } from '@/shell/useDesktopLogs';

type DesktopShellDeveloperLogsProps = {
  api: Pick<DesktopApi, 'readDesktopLogs'>;
};

// #978: 開発者 section が表示され、かつ開発者モード ON のときだけ mount される。
// mount で 1 回取得し、以後は「更新」でだけ再取得する。
export function DesktopShellDeveloperLogs({ api }: DesktopShellDeveloperLogsProps) {
  const logs = useDesktopLogs(api, true);
  return (
    <DeveloperLogViewer
      status={logs.status}
      view={logs.view}
      errorMessage={logs.errorMessage}
      onRefresh={() => void logs.refresh()}
    />
  );
}
