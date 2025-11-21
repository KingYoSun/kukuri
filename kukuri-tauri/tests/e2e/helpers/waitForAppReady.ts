import { $, browser } from '@wdio/globals';

export async function waitForAppReady(timeout = 30000): Promise<void> {
  const startedAt = Date.now();
  let lastSnapshot: {
    status: unknown;
    attrStatus: string | null;
    pageError: string | null;
    htmlProbe: string | null;
    channelReady: boolean;
    readyState: string;
    scripts: string[];
    bootstrapError?: string;
  } | null = null;
  let bootstrapAttempted = false;

  while (Date.now() - startedAt < timeout) {
    const root = await $('#root');
    const exists = await root.isExisting();
    const isDisplayed = exists ? await root.isDisplayed() : false;

    const bridgeStatus = await browser.execute<{
      status: unknown;
      attrStatus: string | null;
      pageError: string | null;
      htmlProbe: string | null;
      channelReady: boolean;
      readyState: string;
      scripts: string[];
    }>(() => {
      const channel = document.getElementById('kukuri-e2e-channel');
      const doc = document.documentElement;
      return {
        status: (window as Record<string, unknown>).__KUKURI_E2E_STATUS__ ?? null,
        attrStatus: doc?.getAttribute('data-kukuri-e2e-status') ?? null,
        pageError: doc?.getAttribute('data-kukuri-e2e-error') ?? null,
        htmlProbe: doc?.getAttribute('data-kukuri-e2e-html') ?? null,
        channelReady: channel?.getAttribute('data-e2e-ready') === '1',
        readyState: document.readyState,
        scripts: Array.from(document.scripts ?? []).map((script) => script.src || script.textContent || ''),
      };
    });

    lastSnapshot = bridgeStatus;
    if (
      !bootstrapAttempted &&
      !bridgeStatus.channelReady &&
      !bridgeStatus.status &&
      typeof browser.execute === 'function'
    ) {
      const bootstrapResult = await browser.execute<{
        triggered: boolean;
        error?: string;
        status?: unknown;
      }>(() => {
        const bootstrap = (window as Record<string, unknown>).__KUKURI_E2E_BOOTSTRAP__;
        try {
          if (typeof bootstrap === 'function') {
            bootstrap();
            return {
              triggered: true,
              status: (window as Record<string, unknown>).__KUKURI_E2E_STATUS__ ?? null,
            };
          }
          return { triggered: false };
        } catch (error) {
          return {
            triggered: true,
            error: error instanceof Error ? error.message : String(error),
          };
        }
      });
      bootstrapAttempted = bootstrapResult.triggered || bootstrapAttempted;
      if (bootstrapResult.error) {
        lastSnapshot = { ...bridgeStatus, bootstrapError: bootstrapResult.error };
      }
    }

    const status = bridgeStatus?.status
      ? String(bridgeStatus.status)
      : bridgeStatus?.attrStatus
        ? String(bridgeStatus.attrStatus)
        : null;
    if (isDisplayed && (status === 'registered' || status === 'disabled' || bridgeStatus.channelReady)) {
      return;
    }

    await browser.pause(500);
  }

  let bundleHasE2E: boolean | 'unknown' = 'unknown';
  const scriptSources = lastSnapshot?.scripts ?? [];
  if (scriptSources[0]) {
    bundleHasE2E = await browser.executeAsync<boolean, [string]>((src, done) => {
      fetch(src)
        .then((response) => response.text())
        .then((text) => done(text.includes('__KUKURI_E2E_STATUS__')))
        .catch(() => done(false));
    }, scriptSources[0]);
  }

  throw new Error(
    `Timed out waiting for Kukuri root container (status=${String(lastSnapshot?.status ?? 'unknown')}, attrStatus=${
      lastSnapshot?.attrStatus ?? 'unknown'
    }, channelReady=${lastSnapshot?.channelReady ?? false}, readyState=${lastSnapshot?.readyState ?? 'n/a'}, htmlProbe=${
      lastSnapshot?.htmlProbe ?? 'missing'
    }, pageError=${lastSnapshot?.pageError ?? 'none'}, scripts=${scriptSources.length}, bundleHasE2E=${bundleHasE2E}${
      lastSnapshot?.bootstrapError ? `, bootstrapError=${lastSnapshot.bootstrapError}` : ''
    })`,
  );
}
