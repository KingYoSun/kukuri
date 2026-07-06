import { describe, expect, it } from 'vitest';

import {
  BACKEND_UNAVAILABLE_MESSAGE,
  InvokeError,
  isBridgeUnavailableError,
  normalizeInvokeError,
} from './error';

describe('normalizeInvokeError', () => {
  it('carries the code from a structured backend error envelope', () => {
    const error = normalizeInvokeError({ code: 'command_failed', message: 'reply target missing' });
    expect(error).toBeInstanceOf(InvokeError);
    expect(error.code).toBe('command_failed');
    expect(error.message).toBe('reply target missing');
  });

  it('maps tauri bridge runtime errors to bridge_unavailable with the sentinel message', () => {
    const error = normalizeInvokeError(new Error('window.__TAURI_INTERNALS__ is undefined'));
    expect(error.code).toBe('bridge_unavailable');
    expect(error.message).toBe(BACKEND_UNAVAILABLE_MESSAGE);
    expect(isBridgeUnavailableError(error)).toBe(true);
  });

  it('treats plain string rejections as command failures and preserves the message', () => {
    const error = normalizeInvokeError('reply target missing');
    expect(error.code).toBe('command_failed');
    expect(error.message).toBe('reply target missing');
    expect(isBridgeUnavailableError(error)).toBe(false);
  });

  it('falls back to bridge_unavailable for unreadable rejections', () => {
    const error = normalizeInvokeError(undefined);
    expect(error.code).toBe('bridge_unavailable');
    expect(error.message).toBe(BACKEND_UNAVAILABLE_MESSAGE);
  });

  it('returns an existing InvokeError unchanged', () => {
    const original = new InvokeError('command_failed', 'already normalized');
    expect(normalizeInvokeError(original)).toBe(original);
  });
});

describe('isBridgeUnavailableError', () => {
  it('is false for non-InvokeError values', () => {
    expect(isBridgeUnavailableError(new Error(BACKEND_UNAVAILABLE_MESSAGE))).toBe(false);
    expect(isBridgeUnavailableError('bridge_unavailable')).toBe(false);
  });
});
