import { useTranslation } from 'react-i18next';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import type { TimelineUpdateMode } from '@/stores/uiStore';
import { cn } from '@/lib/utils';

interface TimelineModeToggleProps {
  mode: TimelineUpdateMode;
  onChange: (mode: TimelineUpdateMode) => void;
  className?: string;
}

export function TimelineModeToggle({ mode, onChange, className }: TimelineModeToggleProps) {
  const { t } = useTranslation();

  return (
    <div className={cn('flex items-center gap-2', className)} data-testid="timeline-mode-toggle">
      <span className="text-xs font-medium text-muted-foreground">
        {t('topics.timelineModeLabel')}
      </span>
      <div className="inline-flex rounded-md border border-border bg-muted/30 p-1">
        <Button
          type="button"
          size="sm"
          variant={mode === 'standard' ? 'secondary' : 'ghost'}
          className="h-8 px-3"
          onClick={() => onChange('standard')}
          data-testid="timeline-mode-toggle-standard"
        >
          {t('topics.timelineModeStandard')}
        </Button>
        <Button
          type="button"
          size="sm"
          variant={mode === 'realtime' ? 'secondary' : 'ghost'}
          className="h-8 px-3"
          onClick={() => onChange('realtime')}
          data-testid="timeline-mode-toggle-realtime"
        >
          {t('topics.timelineModeRealtime')}
          {mode === 'realtime' ? (
            <Badge className="ml-2 bg-emerald-500/15 text-emerald-700 hover:bg-emerald-500/15 dark:text-emerald-300">
              {t('topics.timelineModeLive')}
            </Badge>
          ) : null}
        </Button>
      </div>
    </div>
  );
}
