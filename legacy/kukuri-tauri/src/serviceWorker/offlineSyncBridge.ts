import { errorHandler } from '@/lib/errorHandler';

export const OFFLINE_SYNC_CHANNEL = 'offline-sync';

const workerUrl = new URL('./offlineSyncWorker.ts', import.meta.url);
const workerScope = workerUrl.pathname.replace(/[^/]*$/, '');

let registrationPromise: Promise<ServiceWorkerRegistration | null> | null = null;

function hasServiceWorkerSupport(): boolean {
  return typeof navigator !== 'undefined' && 'serviceWorker' in navigator;
}

export async function registerOfflineSyncWorker(): Promise<ServiceWorkerRegistration | null> {
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
        errorHandler.log('OfflineSync.workerRegisterFailed', err, {
          metadata: { scope: 'registerOfflineSyncWorker' },
        });
        return null;
      }
    })();
  }

  return registrationPromise;
}

export interface OfflineSyncJobPayload {
  jobId?: string;
  userPubkey?: string;
  reason?: string;
  requestedAt?: string;
  retryCount?: number;
  maxRetries?: number;
  retryDelayMs?: number;
  delayMs?: number;
}

function generateJobId(): string {
  if (typeof crypto !== 'undefined' && 'randomUUID' in crypto) {
    return crypto.randomUUID();
  }
  return `offline-sync-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
}

export async function enqueueOfflineSyncJob(
  payload: OfflineSyncJobPayload,
): Promise<string | null> {
  if (!hasServiceWorkerSupport()) {
    return null;
  }

  const registration = await registerOfflineSyncWorker();
  const controller =
    navigator.serviceWorker?.controller ??
    registration?.active ??
    (await navigator.serviceWorker.ready).active;

  if (!controller) {
    return null;
  }

  const jobId = payload.jobId ?? generateJobId();
  controller.postMessage({
    type: 'offline-sync:enqueue',
    payload: {
      ...payload,
      jobId,
      requestedAt: payload.requestedAt ?? new Date().toISOString(),
    },
  });
  return jobId;
}

export function cancelOfflineSyncJob(jobId: string) {
  if (!hasServiceWorkerSupport() || !navigator.serviceWorker?.controller) {
    return;
  }

  navigator.serviceWorker.controller.postMessage({
    type: 'offline-sync:cancel',
    payload: { jobId },
  });
}
