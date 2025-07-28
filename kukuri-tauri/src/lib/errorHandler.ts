import { toast } from 'sonner';

export interface ErrorLogOptions {
  showToast?: boolean;
  toastTitle?: string;
  context?: string;
}

class ErrorHandler {
  private isDevelopment = import.meta.env.DEV;
  private isTest = import.meta.env.MODE === 'test' || process.env.NODE_ENV === 'test';

  log(message: string, error?: unknown, options?: ErrorLogOptions): void {
    // テスト環境では何もしない（テストエラーとの混同を避けるため）
    if (this.isTest) {
      return;
    }

    // 開発環境のみコンソールに出力
    if (this.isDevelopment) {
      // console.warnを使用（console.errorは使わない）
      console.warn(`[ERROR] ${options?.context || 'App'}: ${message}`, error);
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

  info(message: string, context?: string): void {
    if (this.isTest) {
      return;
    }

    if (this.isDevelopment) {
      console.info(`[INFO] ${context || 'App'}: ${message}`);
    }
  }
}

export const errorHandler = new ErrorHandler();