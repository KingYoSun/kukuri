import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Skeleton } from '@/components/ui/skeleton';

interface SummaryMetricCardProps {
  label: string;
  value: string | null;
  helperText?: string | null;
  isLoading?: boolean;
  testId?: string;
}

export function SummaryMetricCard({
  label,
  value,
  helperText,
  isLoading = false,
  testId,
}: SummaryMetricCardProps) {
  return (
    <Card data-testid={testId}>
      <CardHeader className="pb-2">
        <CardTitle className="text-sm font-medium text-muted-foreground">{label}</CardTitle>
      </CardHeader>
      <CardContent className="space-y-1">
        {isLoading ? (
          <Skeleton className="h-7 w-24" />
        ) : (
          <p className="text-2xl font-semibold leading-none">{value ?? '―'}</p>
        )}
        {helperText ? (
          <p className="text-xs text-muted-foreground" data-testid={`${testId}-helper`}>
            {helperText}
          </p>
        ) : null}
      </CardContent>
    </Card>
  );
}
