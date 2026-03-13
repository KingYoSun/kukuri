/// <reference lib="webworker" />

export {};

const CHANNEL_NAME = 'profile-avatar-sync';

type ProfileAvatarSyncJob = {
  jobId: string;
  npub: string;
  knownDocVersion: number | null;
  source: string;
  requestedAt: string;
  retryCount: number;
  force?: boolean;
};

type ChannelMessage =
  | { type: 'profile-avatar-sync:process'; payload: ProfileAvatarSyncJob }
  | { type: 'profile-avatar-sync:complete'; payload: { jobId: string; success: boolean } };

const jobs = new Map<string, ProfileAvatarSyncJob>();
const retryTimers = new Map<string, ReturnType<typeof setTimeout>>();
const channel = new BroadcastChannel(CHANNEL_NAME);
const MAX_RETRY_ATTEMPTS = 3;
const BASE_RETRY_DELAY_MS = 15_000;
const MAX_RETRY_DELAY_MS = 5 * 60 * 1000;

declare const self: ServiceWorkerGlobalScope;

function dispatchJob(job: ProfileAvatarSyncJob) {
  if (retryTimers.has(job.jobId)) {
    const timerId = retryTimers.get(job.jobId);
    if (timerId) {
      clearTimeout(timerId);
    }
    retryTimers.delete(job.jobId);
  }
  channel.postMessage({
    type: 'profile-avatar-sync:process',
    payload: job,
  } satisfies ChannelMessage);
}

function ensureJob(payload: Partial<ProfileAvatarSyncJob>): ProfileAvatarSyncJob {
  const jobId =
    payload.jobId ??
    self.crypto?.randomUUID?.() ??
    `profile-avatar-sync-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
  const job: ProfileAvatarSyncJob = {
    jobId,
    npub: payload.npub ?? '',
    knownDocVersion: payload.knownDocVersion ?? null,
    source: payload.source ?? 'service-worker',
    requestedAt: payload.requestedAt ?? new Date().toISOString(),
    retryCount: payload.retryCount ?? 0,
    force: payload.force,
  };
  return job;
}

self.addEventListener('install', (event) => {
  event.waitUntil(self.skipWaiting());
});

self.addEventListener('activate', (event) => {
  event.waitUntil(self.clients.claim());
});

self.addEventListener('message', (event: ExtendableMessageEvent) => {
  const data = event.data as { type?: string; payload?: Partial<ProfileAvatarSyncJob> } | undefined;
  if (!data || !data.type) {
    return;
  }

  switch (data.type) {
    case 'profile-avatar-sync:enqueue': {
      const job = ensureJob(data.payload ?? {});
      jobs.set(job.jobId, job);
      dispatchJob(job);
      break;
    }
    case 'profile-avatar-sync:retry': {
      if (!data.payload?.jobId) {
        break;
      }
      const existing = jobs.get(data.payload.jobId);
      if (existing) {
        existing.retryCount += 1;
        dispatchJob(existing);
      }
      break;
    }
    default:
      break;
  }
});

channel.addEventListener('message', (event) => {
  const message = event.data as ChannelMessage | undefined;
  if (!message) {
    return;
  }

  if (message.type !== 'profile-avatar-sync:complete') {
    return;
  }

  const { jobId, success } = message.payload;
  if (success) {
    jobs.delete(jobId);
    if (retryTimers.has(jobId)) {
      const timerId = retryTimers.get(jobId);
      if (timerId) {
        clearTimeout(timerId);
      }
      retryTimers.delete(jobId);
    }
    return;
  }

  const job = jobs.get(jobId);
  if (!job) {
    return;
  }

  if (job.retryCount >= MAX_RETRY_ATTEMPTS) {
    jobs.delete(jobId);
    return;
  }

  job.retryCount += 1;
  const delay = Math.min(
    BASE_RETRY_DELAY_MS * Math.pow(2, Math.max(0, job.retryCount - 1)),
    MAX_RETRY_DELAY_MS,
  );
  const timerId = setTimeout(() => {
    dispatchJob(job);
  }, delay);
  retryTimers.set(jobId, timerId);
});
