import { browser } from '@wdio/globals';

export interface VisibleToast {
  text: string;
  type: string | null;
}

type ToastPattern = string | RegExp;

const normalizeToastText = (value: string): string => value.replace(/\s+/g, ' ').trim();

const matchesPattern = (text: string, pattern: ToastPattern): boolean =>
  typeof pattern === 'string' ? text.includes(pattern) : pattern.test(text);

export async function getVisibleToasts(): Promise<VisibleToast[]> {
  return await browser.execute(() => {
    const nodes = Array.from(document.querySelectorAll('[data-sonner-toast]'));
    return nodes
      .map((node) => ({
        text: (node.textContent ?? '').replace(/\s+/g, ' ').trim(),
        type: node.getAttribute('data-type'),
      }))
      .filter((toast) => toast.text.length > 0);
  });
}

export async function waitForToastMatching(options: {
  pattern: ToastPattern;
  timeoutMs?: number;
  intervalMs?: number;
  description: string;
}): Promise<VisibleToast> {
  const timeoutMs = options.timeoutMs ?? 10000;
  const intervalMs = options.intervalMs ?? 200;
  let matched: VisibleToast | null = null;

  await browser.waitUntil(
    async () => {
      const toasts = await getVisibleToasts();
      matched =
        toasts.find((toast) => matchesPattern(normalizeToastText(toast.text), options.pattern)) ??
        null;
      return matched !== null;
    },
    {
      timeout: timeoutMs,
      interval: intervalMs,
      timeoutMsg: `Toast did not appear: ${options.description}`,
    },
  );

  if (!matched) {
    throw new Error(`Toast did not appear: ${options.description}`);
  }
  return matched;
}

export async function expectNoToastMatching(options: {
  patterns: ToastPattern[];
  durationMs?: number;
  intervalMs?: number;
  description: string;
}): Promise<void> {
  const durationMs = options.durationMs ?? 4000;
  const intervalMs = options.intervalMs ?? 200;
  const deadline = Date.now() + durationMs;

  while (Date.now() < deadline) {
    const toasts = await getVisibleToasts();
    const matched = toasts.find((toast) =>
      options.patterns.some((pattern) => matchesPattern(normalizeToastText(toast.text), pattern)),
    );
    if (matched) {
      throw new Error(
        `Unexpected toast appeared (${options.description}): ${JSON.stringify(matched)}`,
      );
    }
    await browser.pause(intervalMs);
  }
}

export async function waitForToastsToClear(timeoutMs = 10000): Promise<void> {
  await browser.waitUntil(async () => (await getVisibleToasts()).length === 0, {
    timeout: timeoutMs,
    interval: 200,
    timeoutMsg: 'Visible toasts did not clear in time',
  });
}
