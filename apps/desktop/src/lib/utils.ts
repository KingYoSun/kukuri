import { clsx, type ClassValue } from 'clsx';
import { twMerge } from 'tailwind-merge';

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

export const CLIPBOARD_COPY_EVENT = 'kukuri:clipboard-copy';

export async function copyTextToClipboard(value: string): Promise<boolean> {
  if (typeof navigator === 'undefined' || !navigator.clipboard?.writeText) {
    return false;
  }
  await navigator.clipboard.writeText(value);
  if (typeof window !== 'undefined' && typeof window.dispatchEvent === 'function') {
    window.dispatchEvent(
      new CustomEvent(CLIPBOARD_COPY_EVENT, {
        detail: { value },
      })
    );
  }
  return true;
}
