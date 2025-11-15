import { errorHandler } from '@/lib/errorHandler';

export const PROFILE_AVATAR_SYNC_CHANNEL = 'profile-avatar-sync';

const workerUrl = new URL('./profileAvatarSyncSW.ts', import.meta.url);
const workerScope = new URL('./', workerUrl).pathname;

let registrationPromise: Promise<ServiceWorkerRegistration | null> | null = null;

function hasServiceWorkerSupport(): boolean {
  return typeof navigator !== 'undefined' && 'serviceWorker' in navigator;
}

export async function registerProfileAvatarSyncWorker(): Promise<ServiceWorkerRegistration | null> {
  if (!hasServiceWorkerSupport()) {
    return null;
  }

  if (!registrationPromise) {
    registrationPromise = (async () => {
      try {
        const registration = await navigator.serviceWorker.register(workerUrl, {
          scope: workerScope,
          type: 'module',
        });
        return registration;
      } catch (err) {
        errorHandler.log('ProfileAvatarSync.workerRegisterFailed', err, {
          metadata: { scope: 'registerProfileAvatarSyncWorker' },
        });
        return null;
      }
    })();
  }

  return registrationPromise;
}

export interface ProfileAvatarSyncJobPayload {
  jobId?: string;
  npub: string;
  knownDocVersion?: number | null;
  source?: string;
  requestedAt?: string;
  retryCount?: number;
  force?: boolean;
}

function createJobId() {
  if (typeof crypto !== 'undefined' && 'randomUUID' in crypto) {
    return crypto.randomUUID();
  }
  return `profile-avatar-sync-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
}

export async function enqueueProfileAvatarSyncJob(
  payload: ProfileAvatarSyncJobPayload,
): Promise<string | null> {
  if (!hasServiceWorkerSupport()) {
    return null;
  }

  const registration = await registerProfileAvatarSyncWorker();
  const controller =
    navigator.serviceWorker?.controller ??
    registration?.active ??
    (await navigator.serviceWorker.ready).active;

  if (!controller) {
    return null;
  }

  const jobId = payload.jobId ?? createJobId();
  controller.postMessage({
    type: 'profile-avatar-sync:enqueue',
    payload: {
      ...payload,
      jobId,
      requestedAt: payload.requestedAt ?? new Date().toISOString(),
      retryCount: payload.retryCount ?? 0,
    },
  });

  return jobId;
}
