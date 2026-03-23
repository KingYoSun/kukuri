import { Badge } from '@/components/ui/badge';

type StatusBadgeTone = 'neutral' | 'accent' | 'warning' | 'destructive';

type StatusBadgeProps = {
  label: string;
  tone?: StatusBadgeTone;
};

export function StatusBadge({ label, tone = 'neutral' }: StatusBadgeProps) {
  return <Badge tone={tone}>{label}</Badge>;
}
