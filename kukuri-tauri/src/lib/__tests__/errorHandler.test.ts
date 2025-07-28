import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { ErrorHandler } from '../errorHandler';
import { toast } from 'sonner';

vi.mock('sonner', () => ({
  toast: {
    error: vi.fn(),
  },
}));

describe('errorHandler', () => {
  const originalEnv = import.meta.env;
  let errorHandler: ErrorHandler;

  beforeEach(() => {
    vi.clearAllMocks();
    // console メソッドをモック
    vi.spyOn(console, 'warn').mockImplementation(() => {});
    vi.spyOn(console, 'info').mockImplementation(() => {});
  });

  afterEach(() => {
    vi.restoreAllMocks();
    import.meta.env = originalEnv;
  });

  describe('in test environment', () => {
    beforeEach(() => {
      // テスト環境をシミュレート
      import.meta.env = {
        ...originalEnv,
        MODE: 'test',
        DEV: false,
      };
      errorHandler = new ErrorHandler();
      errorHandler.setTestEnvironment('test');
    });

    it('should not log anything in test environment', () => {
      errorHandler.log('Test error', new Error('Test'));

      expect(console.warn).not.toHaveBeenCalled();
      expect(toast.error).not.toHaveBeenCalled();
    });

    it('should not warn in test environment', () => {
      errorHandler.warn('Test warning');

      expect(console.warn).not.toHaveBeenCalled();
    });

    it('should not log info in test environment', () => {
      errorHandler.info('Test info');

      expect(console.info).not.toHaveBeenCalled();
    });
  });

  describe('in development environment', () => {
    beforeEach(() => {
      // 開発環境をシミュレート
      import.meta.env = {
        ...originalEnv,
        MODE: 'development',
        DEV: true,
      };
      errorHandler = new ErrorHandler();
      errorHandler.setTestEnvironment('development');
    });

    it('should log to console in development', () => {
      const error = new Error('Test error');
      errorHandler.log('Something went wrong', error, { context: 'TestComponent' });

      expect(console.warn).toHaveBeenCalledWith(
        '[ERROR] TestComponent: Something went wrong',
        error,
      );
    });

    it('should show toast when showToast is true', () => {
      errorHandler.log('Connection failed', undefined, {
        showToast: true,
        toastTitle: '接続エラー',
      });

      expect(toast.error).toHaveBeenCalledWith('接続エラー', {
        description: 'Connection failed',
      });
    });

    it('should use default toast title when not provided', () => {
      errorHandler.log('Something failed', undefined, {
        showToast: true,
      });

      expect(toast.error).toHaveBeenCalledWith('エラーが発生しました', {
        description: 'Something failed',
      });
    });

    it('should log warnings', () => {
      errorHandler.warn('This is a warning', 'WarningContext');

      expect(console.warn).toHaveBeenCalledWith('[WARN] WarningContext: This is a warning');
    });

    it('should log info', () => {
      errorHandler.info('This is info', 'InfoContext');

      expect(console.info).toHaveBeenCalledWith('[INFO] InfoContext: This is info');
    });
  });

  describe('in production environment', () => {
    beforeEach(() => {
      // 本番環境をシミュレート
      import.meta.env = {
        ...originalEnv,
        MODE: 'production',
        DEV: false,
      };
      errorHandler = new ErrorHandler();
      errorHandler.setTestEnvironment('production');
    });

    it('should not log to console in production', () => {
      errorHandler.log('Production error');

      expect(console.warn).not.toHaveBeenCalled();
    });

    it('should still show toast in production when requested', () => {
      errorHandler.log('Production error', undefined, {
        showToast: true,
      });

      expect(toast.error).toHaveBeenCalled();
    });
  });
});
