import { beforeEach, afterEach, describe, expect, it, vi } from 'vitest';

type MessageHandler = (event: { data: any }) => void;

describe('profileAvatarSync Service Worker', () => {
  let messageHandlers: MessageHandler[];
  let channelListeners: MessageHandler[];
  let broadcastInstance: {
    postMessage: ReturnType<typeof vi.fn>;
  } | null;

  beforeEach(async () => {
    vi.useFakeTimers();
    vi.resetModules();
    messageHandlers = [];
    channelListeners = [];
    broadcastInstance = null;

    vi.stubGlobal('self', {
      addEventListener: (type: string, handler: MessageHandler) => {
        if (type === 'message') {
          messageHandlers.push(handler);
        }
      },
      skipWaiting: vi.fn(),
      clients: {
        claim: vi.fn(),
      },
      crypto: {
        randomUUID: vi.fn(() => 'job-random'),
      },
    });

    class StubBroadcastChannel {
      name: string;
      postMessage = vi.fn();

      constructor(name: string) {
        this.name = name;
        broadcastInstance = this;
      }

      addEventListener(type: string, handler: MessageHandler) {
        if (type === 'message') {
          channelListeners.push(handler);
        }
      }

      removeEventListener() {
        // no-op for tests
      }

      close() {
        // no-op
      }
    }

    vi.stubGlobal('BroadcastChannel', StubBroadcastChannel);

    await import('@/serviceWorker/profileAvatarSyncSW');
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
    Reflect.deleteProperty(globalThis, 'self');
    Reflect.deleteProperty(globalThis, 'BroadcastChannel');
  });

  it('re-enqueues failed jobs with incremental retry delay', () => {
    const enqueueHandler = messageHandlers[0];
    expect(enqueueHandler).toBeDefined();
    const setTimeoutSpy = vi.spyOn(globalThis, 'setTimeout');
    enqueueHandler({
      data: {
        type: 'profile-avatar-sync:enqueue',
        payload: {
          jobId: 'job-1',
          npub: 'npub1example',
          knownDocVersion: 3,
          requestedAt: '2025-11-12T00:00:00Z',
          retryCount: 0,
        },
      },
    });

    expect(broadcastInstance?.postMessage).toHaveBeenCalledTimes(1);

    const completeListener = channelListeners[0];
    expect(completeListener).toBeDefined();

    completeListener({
      data: {
        type: 'profile-avatar-sync:complete',
        payload: { jobId: 'job-1', success: false },
      },
    });

    expect(setTimeoutSpy).toHaveBeenCalledWith(expect.any(Function), 15_000);
    vi.advanceTimersByTime(15_000);

    expect(broadcastInstance?.postMessage).toHaveBeenCalledTimes(2);
    const retryPayload = (broadcastInstance?.postMessage as vi.Mock).mock.calls[1][0]
      .payload;
    expect(retryPayload.retryCount).toBe(1);
  });

  it('stops scheduling retries after success', () => {
    const enqueueHandler = messageHandlers[0];
    enqueueHandler({
      data: {
        type: 'profile-avatar-sync:enqueue',
        payload: {
          jobId: 'job-2',
          npub: 'npub1example',
          knownDocVersion: null,
          requestedAt: '2025-11-12T00:00:00Z',
          retryCount: 0,
        },
      },
    });

    const completeListener = channelListeners[0];
    completeListener({
      data: {
        type: 'profile-avatar-sync:complete',
        payload: { jobId: 'job-2', success: true },
      },
    });

    vi.advanceTimersByTime(5 * 60 * 1000);
    expect(broadcastInstance?.postMessage).toHaveBeenCalledTimes(1);
  });
});
