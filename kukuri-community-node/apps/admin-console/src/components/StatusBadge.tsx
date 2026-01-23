type StatusBadgeProps = {
  status?: string | null;
  label?: string;
};

const toneForStatus = (status: string) => {
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
  return '';
};

export const StatusBadge = ({ status, label }: StatusBadgeProps) => {
  const safeStatus = status ?? 'unknown';
  const tone = toneForStatus(safeStatus);
  const className = `badge${tone ? ` ${tone}` : ''}`;
  return <span className={className}>{label ?? safeStatus}</span>;
};
