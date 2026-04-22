import { useCallback, useRef } from 'react';

type LoadTopicsArgs = readonly [string[], string, string | null];
type LoadTopicsWaiter = { resolve: () => void; reject: (error: unknown) => void };

export function useQueuedLoadTopics(
  runLoadTopics: (topics: string[], activeTopic: string, currentThread: string | null) => Promise<void>
) {
  const loadTopicsInFlightRef = useRef(false);
  const queuedLoadTopicsArgsRef = useRef<LoadTopicsArgs | null>(null);
  const loadTopicsWaitersRef = useRef<LoadTopicsWaiter[]>([]);

  const drainLoadTopicsQueue = useCallback(
    async (initialArgs: LoadTopicsArgs) => {
      let nextArgs: LoadTopicsArgs | null = initialArgs;
      let lastError: unknown = null;

      while (nextArgs) {
        queuedLoadTopicsArgsRef.current = null;
        try {
          await runLoadTopics(...nextArgs);
          lastError = null;
        } catch (error) {
          lastError = error;
        }
        nextArgs = queuedLoadTopicsArgsRef.current;
      }

      loadTopicsInFlightRef.current = false;
      const waiters = loadTopicsWaitersRef.current;
      loadTopicsWaitersRef.current = [];
      for (const waiter of waiters) {
        if (lastError) {
          waiter.reject(lastError);
          continue;
        }
        waiter.resolve();
      }
    },
    [runLoadTopics]
  );

  return useCallback(
    (currentTopics: string[], currentActiveTopic: string, currentThread: string | null) => {
      const args: LoadTopicsArgs = [[...currentTopics], currentActiveTopic, currentThread];
      return new Promise<void>((resolve, reject) => {
        loadTopicsWaitersRef.current.push({ resolve, reject });
        if (loadTopicsInFlightRef.current) {
          queuedLoadTopicsArgsRef.current = args;
          return;
        }
        loadTopicsInFlightRef.current = true;
        void drainLoadTopicsQueue(args);
      });
    },
    [drainLoadTopicsQueue]
  );
}
