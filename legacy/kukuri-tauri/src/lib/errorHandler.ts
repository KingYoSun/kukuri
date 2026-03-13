import { toast } from 'sonner';

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
    return import.meta.env.DEV;
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

    if (this.isDevelopment) {
      console.warn(`[WARN] ${context || 'App'}: ${message}`);
    }
  }

  info(message: string, context?: string, metadata?: Record<string, unknown>): void {
    if (this.isTest) {
      return;
    }

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
