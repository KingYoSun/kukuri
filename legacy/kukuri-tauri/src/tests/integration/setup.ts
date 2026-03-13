import { vi, Mock } from 'vitest';
import { invoke } from '@tauri-apps/api/core';

// zustandのモックを解除して実際の実装を使用
vi.unmock('zustand');
vi.unmock('zustand/middleware');

// Tauriコマンドのモック用のレスポンスを定義
export const mockTauriResponses = new Map<string, unknown | (() => unknown)>();

// Tauriコマンドをモック
export function setupTauriMocks() {
  const mockInvoke = invoke as Mock;

  mockInvoke.mockImplementation(async (cmd: string, args?: unknown) => {
    // モックレスポンスが設定されている場合はそれを返す
    if (mockTauriResponses.has(cmd)) {
      const response = mockTauriResponses.get(cmd);
      // 関数の場合は実行する
      if (typeof response === 'function') {
        return await response();
      }
      // Promise.rejectの場合は、適切にawaitして例外を投げる
      if (response instanceof Promise) {
        return await response;
      }
      return response;
    }

    // デフォルトのモックレスポンス
    switch (cmd) {
      case 'generate_keypair':
        return {
          publicKey: 'npub1testpublickey123456789abcdef',
          secretKey: 'nsec1testsecretkey123456789abcdef',
        };

      case 'get_public_key':
        return 'npub1testpublickey123456789abcdef';

      case 'initialize_nostr':
        return { success: true };

      case 'list_topics':
        return [
          { id: 1, name: 'test-topic-1', description: 'Test Topic 1' },
          { id: 2, name: 'test-topic-2', description: 'Test Topic 2' },
        ];

      case 'create_topic': {
        const topicArgs = args as { name?: string; description?: string } | undefined;
        return {
          id: 3,
          name: topicArgs?.name || 'new-topic',
          description: topicArgs?.description || 'New Topic',
        };
      }

      case 'list_posts':
        return [
          {
            id: 'post1',
            content: 'Test post 1',
            pubkey: 'npub1testpublickey123456789abcdef',
            created_at: Date.now() / 1000,
            tags: [],
          },
          {
            id: 'post2',
            content: 'Test post 2',
            pubkey: 'npub1testpublickey123456789abcdef',
            created_at: Date.now() / 1000,
            tags: [['t', 'test-topic-1']],
          },
        ];

      case 'create_post': {
        const postArgs = args as { content?: string; tags?: string[][] } | undefined;
        return {
          id: 'newpost123',
          content: postArgs?.content || 'New post',
          pubkey: 'npub1testpublickey123456789abcdef',
          created_at: Date.now() / 1000,
          tags: postArgs?.tags || [],
        };
      }

      case 'connect_relay': {
        const connectArgs = args as { url?: string } | undefined;
        return { connected: true, url: connectArgs?.url || 'wss://relay.example.com' };
      }

      case 'disconnect_relay': {
        const disconnectArgs = args as { url?: string } | undefined;
        return { disconnected: true, url: disconnectArgs?.url || 'wss://relay.example.com' };
      }

      case 'get_relay_status':
        return {
          'wss://relay.example.com': 'connected',
          'wss://relay2.example.com': 'disconnected',
        };

      case 'disconnect_nostr':
        return { success: true };

      case 'logout':
        return { success: true };

      // Secure Storage commands
      case 'add_account':
        return { npub: 'npub1' + Math.random().toString(36).substring(7), pubkey: 'pubkey123' };

      case 'list_accounts':
        return [];

      case 'get_current_account':
        return null;

      case 'secure_login': {
        const loginArgs = args as { npub: string } | undefined;
        return {
          public_key: 'pubkey_' + loginArgs?.npub,
          npub: loginArgs?.npub || 'npub1test',
        };
      }

      case 'remove_account':
        return { success: true };

      default:
        throw new Error(`Unknown command: ${cmd}`);
    }
  });
}

// テスト用のユーティリティ関数
export function setMockResponse(command: string, response: unknown) {
  mockTauriResponses.set(command, response);
}

export function clearMockResponses() {
  mockTauriResponses.clear();
}

// 非同期操作を待つヘルパー
export function waitFor(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

// ストレージのモック
export const mockStorage = {
  data: new Map<string, string>(),

  getItem(key: string): string | null {
    return this.data.get(key) || null;
  },

  setItem(key: string, value: string): void {
    this.data.set(key, value);
  },

  removeItem(key: string): void {
    this.data.delete(key);
  },

  clear(): void {
    this.data.clear();
  },
};

// テスト環境のセットアップ
export function setupIntegrationTest() {
  setupTauriMocks();

  // localStorageをモック
  Object.defineProperty(window, 'localStorage', {
    value: mockStorage,
    writable: true,
  });

  // テスト後のクリーンアップ
  return () => {
    clearMockResponses();
    mockStorage.clear();
  };
}
