import { toast } from 'sonner';

export interface ErrorLogOptions {
  showToast?: boolean;
  toastTitle?: string;
  context?: string;
  metadata?: Record<string, unknown>;
}

class ErrorHandler {
  // テスト時に環境を強制的に変更できるようにする
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

  // テスト用メソッド
  setTestEnvironment(env: 'development' | 'production' | 'test' | null) {
    this._forceEnvironment = env;
  }

  log(message: string, error?: unknown, options?: ErrorLogOptions): void {
    // テスト環境では何もしない（テストエラーとの混同を避けるため）
    if (this.isTest) {
      return;
    }

    // 開発環境のみコンソールに出力
    if (this.isDevelopment) {
      // console.warnを使用（console.errorは使わない）
      if (options?.metadata) {
        console.warn(`[ERROR] ${options.context || 'App'}: ${message}`, error, options.metadata);
      } else {
        console.warn(`[ERROR] ${options?.context || 'App'}: ${message}`, error);
      }
    }

    // ユーザーへの通知（オプション）
    if (options?.showToast) {
      toast.error(options.toastTitle || 'エラーが発生しました', {
        description: message,
      });
    }

    // 本番環境では将来的にエラーレポーティングサービスに送信可能
    // if (!this.isDevelopment) {
    //   // Sentry, LogRocket などにエラーを送信
    // }
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

// テスト用のエクスポート
export { ErrorHandler };
