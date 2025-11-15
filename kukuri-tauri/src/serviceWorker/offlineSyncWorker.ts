/// <reference lib="webworker" />

export {};

const CHANNEL_NAME = 'offline-sync';
const DEFAULT_MAX_RETRIES = 3;
const DEFAULT_RETRY_DELAY_MS = 5_000;

type OfflineSyncJob = {
  jobId: string;
  userPubkey?: string;
  reason?: string;
  requestedAt: string;
  retryCount: number;
  maxRetries: number;
  retryDelayMs: number;
};

type EnqueuePayload = Partial<
  OfflineSyncJob & {
    delayMs?: number;
  }
>;

type WorkerMessage =
  | { type: 'offline-sync:enqueue'; payload?: EnqueuePayload }
  | { type: 'offline-sync:retry'; payload?: { jobId: string } }
  | { type: 'offline-sync:cancel'; payload?: { jobId: string } };

type ChannelMessage =
  | { type: 'offline-sync:process'; payload: OfflineSyncJob }
  | {
      type: 'offline-sync:complete';
      payload: {
        jobId: string;
        success: boolean;
        retryCount: number;
        maxRetries: number;
        retryDelayMs: number;
      };
    }
  | {
      type: 'offline-sync:scheduled';
      payload: {
        jobId: string;
        retryCount: number;
        maxRetries: number;
        retryDelayMs: number;
        nextRunAt: string;
      };
    };

declare const self: ServiceWorkerGlobalScope;

const jobs = new Map<string, OfflineSyncJob>();
const timers = new Map<string, number>();
const channel = new BroadcastChannel(CHANNEL_NAME);

function ensureJob(payload: EnqueuePayload = {}): OfflineSyncJob {
  const jobId =
    payload.jobId ??
    self.crypto?.randomUUID?.() ??
    `offline-sync-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
  return {
    jobId,
    userPubkey: payload.userPubkey,
    reason: payload.reason,
    requestedAt: payload.requestedAt ?? new Date().toISOString(),
    retryCount: payload.retryCount ?? 0,
    maxRetries: payload.maxRetries ?? DEFAULT_MAX_RETRIES,
    retryDelayMs: payload.retryDelayMs ?? DEFAULT_RETRY_DELAY_MS,
  };
}

function clearTimer(jobId: string) {
  const timer = timers.get(jobId);
  if (typeof timer === 'number') {
    self.clearTimeout(timer);
  }
  timers.delete(jobId);
}

function scheduleJob(job: OfflineSyncJob, delayMs: number) {
  clearTimer(job.jobId);
  const safeDelay = Math.max(0, delayMs);
  const handle = self.setTimeout(
    () => {
      timers.delete(job.jobId);
      channel.postMessage({
        type: 'offline-sync:process',
        payload: job,
      } satisfies ChannelMessage);
    },
    safeDelay,
  );
  timers.set(job.jobId, handle as unknown as number);

  channel.postMessage({
    type: 'offline-sync:scheduled',
    payload: {
      jobId: job.jobId,
      retryCount: job.retryCount,
      maxRetries: job.maxRetries,
      retryDelayMs: job.retryDelayMs,
      nextRunAt: new Date(Date.now() + safeDelay).toISOString(),
    },
  } satisfies ChannelMessage);
}

self.addEventListener('install', (event) => {
  event.waitUntil(self.skipWaiting());
});

self.addEventListener('activate', (event) => {
  event.waitUntil(self.clients.claim());
});

self.addEventListener('message', (event: ExtendableMessageEvent) => {
  const data = event.data as WorkerMessage | undefined;
  if (!data) {
    return;
  }

  switch (data.type) {
    case 'offline-sync:enqueue': {
      const job = ensureJob(data.payload);
      jobs.set(job.jobId, job);
      const delay = (data.payload?.delayMs ?? 0) > 0 ? (data.payload?.delayMs ?? 0) : 0;
      scheduleJob(job, delay);
      break;
    }
    case 'offline-sync:retry': {
      const jobId = data.payload?.jobId;
      if (!jobId) {
        break;
      }
      const job = jobs.get(jobId);
      if (!job) {
        break;
      }
      scheduleJob(job, job.retryDelayMs);
      break;
    }
    case 'offline-sync:cancel': {
      const jobId = data.payload?.jobId;
      if (!jobId) {
        break;
      }
      clearTimer(jobId);
      jobs.delete(jobId);
      break;
    }
    default:
      break;
  }
});

channel.addEventListener('message', (event) => {
  const data = event.data as ChannelMessage | undefined;
  if (!data || data.type !== 'offline-sync:complete') {
    return;
  }

  const job = jobs.get(data.payload.jobId);
  if (!job) {
    return;
  }

  if (data.payload.success) {
    clearTimer(job.jobId);
    jobs.delete(job.jobId);
    return;
  }

  if (job.retryCount + 1 >= job.maxRetries) {
    clearTimer(job.jobId);
    jobs.delete(job.jobId);
    return;
  }

  job.retryCount += 1;
  jobs.set(job.jobId, job);
  scheduleJob(job, job.retryDelayMs);
});
