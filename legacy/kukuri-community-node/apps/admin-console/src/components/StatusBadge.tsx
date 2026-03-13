import { Badge } from './ui';

type StatusBadgeProps = {
  status?: string | null;
  label?: string;
};

const toneForStatus = (status: string): 'default' | 'good' | 'warn' | 'bad' => {
  const normalized = status.toLowerCase();
  const good = ['healthy', 'ok', 'active', 'approved', 'enabled', 'current'];
  const warn = ['pending', 'queued', 'degraded', 'inactive', 'disabled'];
  const bad = ['rejected', 'failed', 'error', 'unreachable'];
  if (good.includes(normalized)) {
    return 'good';
  }
  if (warn.includes(normalized)) {
    return 'warn';
  }
  if (bad.includes(normalized)) {
    return 'bad';
  }
  return 'default';
};

export const StatusBadge = ({ status, label }: StatusBadgeProps) => {
  const safeStatus = status ?? 'unknown';
  return <Badge tone={toneForStatus(safeStatus)}>{label ?? safeStatus}</Badge>;
};
