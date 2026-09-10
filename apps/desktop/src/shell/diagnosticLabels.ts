import { resources } from '@/i18n';

type Translate = (key: string, options?: Record<string, unknown>) => string;
type DiagnosticGroup = keyof typeof resources.en.settings.diagnostics.values;

// Display-only conversion. Never feed translated strings back to the protocol or store.
export function diagnosticValueLabel(
  group: DiagnosticGroup, value: string, t: Translate, includeCode = true,
): string {
  if (!value) return t('common:fallbacks.none');
  const known = Object.hasOwn(resources.en.settings.diagnostics.values[group], value);
  const label = t(known ? `settings:diagnostics.values.${group}.${value}` : 'settings:diagnostics.unknownValue');
  return includeCode || !known ? `${label} (${value})` : label;
}

const initialJoinTimeout = /^(?:topic join pending: )?timed out waiting for initial topic join$/;

export function diagnosticErrorSummary(error: string | null | undefined, t: Translate): string | null {
  if (!error) return null;
  const key = initialJoinTimeout.test(error)
    ? 'settings:diagnostics.initialJoinTimeout'
    : 'settings:diagnostics.error';
  return t(key);
}

export function diagnosticErrorLabel(error: string | null | undefined, t: Translate): string | null {
  const summary = diagnosticErrorSummary(error, t);
  return summary ? `${summary} ${t('settings:diagnostics.original', { value: error })}` : null;
}

export function diagnosticStatusDetail(detail: string, t: Translate): string {
  return initialJoinTimeout.test(detail)
    ? diagnosticErrorLabel(detail, t)!
    : `${t('settings:diagnostics.connectionDetail')} ${t('settings:diagnostics.original', { value: detail })}`;
}
