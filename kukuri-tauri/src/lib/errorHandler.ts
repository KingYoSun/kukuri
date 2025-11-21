import { toast } from 'sonner';
import { setE2EDebugMessage } from './utils/e2eDebug';

export interface ErrorLogOptions {
  showToast?: boolean;
  toastTitle?: string;
  context?: string;
  metadata?: Record<string, unknown>;
}

class ErrorHandler {
  private _forceEnvironment: 'development' | 'production' | 'test' | null = null;

  private get isDevelopment() {
    if (this._forceEnvironment) {
      return this._forceEnvironment === 'development';
    }
    const isE2EDebug =
      import.meta.env.TAURI_ENV_DEBUG === 'true' || import.meta.env.VITE_ENABLE_E2E === 'true';
    return import.meta.env.DEV || isE2EDebug;
  }

  private get isTest() {
    if (this._forceEnvironment) {
      return this._forceEnvironment === 'test';
    }
    return import.meta.env.MODE === 'test';
  }

  setTestEnvironment(env: 'development' | 'production' | 'test' | null) {
    this._forceEnvironment = env;
  }

  log(message: string, error?: unknown, options?: ErrorLogOptions): void {
    if (this.isTest) {
      return;
    }

    const errorMessage =
      error instanceof Error ? error.message : error === undefined ? null : String(error);
    let errorDetail: string | null = null;
    if (error) {
      try {
        errorDetail = JSON.stringify(error);
      } catch {
        errorDetail = null;
      }
    }
    setE2EDebugMessage(
      message,
      { level: 'error', context: options?.context, error: errorMessage, detail: errorDetail },
      { key: 'last-log' },
    );

    if (this.isDevelopment) {
      if (options?.metadata) {
        console.warn(`[ERROR] ${options.context || 'App'}: ${message}`, error, options.metadata);
      } else {
        console.warn(`[ERROR] ${options?.context || 'App'}: ${message}`, error);
      }
    }

    if (options?.showToast) {
      toast.error(options.toastTitle || 'エラーが発生しました', {
        description: message,
      });
    }
  }

  warn(message: string, context?: string): void {
    if (this.isTest) {
      return;
    }

    setE2EDebugMessage(message, { level: 'warn', context }, { key: 'last-log' });

    if (this.isDevelopment) {
      console.warn(`[WARN] ${context || 'App'}: ${message}`);
    }
  }

  info(message: string, context?: string, metadata?: Record<string, unknown>): void {
    if (this.isTest) {
      return;
    }

    setE2EDebugMessage(message, { level: 'info', context, ...metadata }, { key: 'last-log' });

    if (this.isDevelopment) {
      if (metadata) {
        console.info(`[INFO] ${context || 'App'}: ${message}`, metadata);
      } else {
        console.info(`[INFO] ${context || 'App'}: ${message}`);
      }
    }
  }
}

export const errorHandler = new ErrorHandler();

export { ErrorHandler };
