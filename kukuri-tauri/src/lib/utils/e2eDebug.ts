const isE2EDebugEnabled =
  import.meta.env.VITE_ENABLE_E2E === 'true' || import.meta.env.TAURI_ENV_DEBUG === 'true';

const clamp = (value: string, max = 500) => (value.length > max ? value.slice(0, max) : value);

export const setE2EDebugAttr = (
  key: string,
  value: string | number | boolean | null | undefined,
) => {
  if (typeof document === 'undefined' || !isE2EDebugEnabled) {
    return;
  }
  const encoded = value === null || value === undefined ? '' : clamp(String(value));
  document.documentElement?.setAttribute(`data-e2e-${key}`, encoded);
};

export const setE2EDebugJson = (key: string, payload: unknown) => {
  if (typeof document === 'undefined' || !isE2EDebugEnabled) {
    return;
  }
  try {
    const serialized = clamp(JSON.stringify(payload ?? ''));
    document.documentElement?.setAttribute(`data-e2e-${key}`, serialized);
  } catch {
    // noop: debug only
  }
};

export const setE2EDebugMessage = (
  message: string,
  metadata?: Record<string, unknown> | null,
  options: { key?: string } = {},
) => {
  if (!isE2EDebugEnabled) {
    return;
  }
  const payload = metadata ? { message, ...metadata } : { message };
  setE2EDebugJson(options.key ?? 'debug', payload);
};

export const setE2EAuthDebug = (state: {
  isAuthenticated: boolean;
  npub?: string | null;
  accounts?: Array<{ npub: string; display_name?: string | null }>;
}) => {
  setE2EDebugJson('auth', {
    ...state,
    timestamp: Date.now(),
  });
};
