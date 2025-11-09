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
const channel = new BroadcastChannel(CHANNEL_NAME);

declare const self: ServiceWorkerGlobalScope;

function dispatchJob(job: ProfileAvatarSyncJob) {
  channel.postMessage({
    type: 'profile-avatar-sync:process',
    payload: job,
  } satisfies ChannelMessage);
}

function ensureJob(payload: Partial<ProfileAvatarSyncJob>): ProfileAvatarSyncJob {
  const jobId =
    payload.jobId ??
    (self.crypto?.randomUUID?.() ??
      `profile-avatar-sync-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`);
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

  if (message.type === 'profile-avatar-sync:complete') {
    jobs.delete(message.payload.jobId);
  }
});
