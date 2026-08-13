export const BACKEND_UNAVAILABLE_MESSAGE = 'Desktop backend is not attached.';

// バックエンド(src-tauri)の CommandError { code, message } と対になる同一バイナリ内契約。
// code はドメイン別に拡充される可能性があるため string で保持する(既知値は下記 2 つ)。
export type InvokeErrorCode = 'command_failed' | 'bridge_unavailable';

export class InvokeError extends Error {
  readonly code: string;
  readonly status: number | null;
  readonly retryAfterSeconds: number | null;

  constructor(
    code: string,
    message: string,
    status: number | null = null,
    retryAfterSeconds: number | null = null
  ) {
    super(message);
    this.name = 'InvokeError';
    this.code = code;
    this.status = status;
    this.retryAfterSeconds = retryAfterSeconds;
  }
}

export function isBridgeUnavailableError(error: unknown): boolean {
  return error instanceof InvokeError && error.code === 'bridge_unavailable';
}

function isCommandErrorPayload(error: unknown): error is {
  code: string;
  message: string;
  status?: number | null;
  retry_after_seconds?: number | null;
} {
  return (
    typeof error === 'object' &&
    error !== null &&
    'code' in error &&
    typeof error.code === 'string' &&
    'message' in error &&
    typeof error.message === 'string'
  );
}

export function normalizeInvokeError(error: unknown): InvokeError {
  if (error instanceof InvokeError) {
    return error;
  }
  // バックエンドの構造化封筒はそのまま code を引き継ぐ(message は平文文言のまま)。
  if (isCommandErrorPayload(error)) {
    return new InvokeError(
      error.code,
      error.message,
      typeof error.status === 'number' ? error.status : null,
      typeof error.retry_after_seconds === 'number' ? error.retry_after_seconds : null
    );
  }
  const message =
    error instanceof Error
      ? error.message
      : typeof error === 'string'
        ? error
        : typeof error === 'object' &&
            error !== null &&
            'message' in error &&
            typeof error.message === 'string'
          ? error.message
          : null;
  if (message === null) {
    // 解読不能な reject は従来どおり「ブリッジ不在」として扱う(旧実装は sentinel
    // message を合成しており、App 側はそれを未接続として解釈していた)。
    return new InvokeError('bridge_unavailable', BACKEND_UNAVAILABLE_MESSAGE);
  }
  const lowered = message.toLowerCase();
  if (
    lowered.includes('__tauri') ||
    lowered.includes('__tauri_ipc__') ||
    (lowered.includes('ipc') && lowered.includes('not available')) ||
    (lowered.includes('invoke') && lowered.includes('undefined'))
  ) {
    // Tauri ブリッジ不在(ブラウザ/mock なし実行)の JS ランタイムエラー。
    // 表示文言は互換のため従来の sentinel を維持し、判定は code で行う。
    return new InvokeError('bridge_unavailable', BACKEND_UNAVAILABLE_MESSAGE);
  }
  return new InvokeError('command_failed', message);
}
