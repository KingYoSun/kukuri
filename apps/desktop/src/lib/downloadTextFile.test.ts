import { afterEach, expect, test, vi } from 'vitest';

import { downloadTextFile } from './downloadTextFile';

afterEach(() => {
  vi.restoreAllMocks();
});

test('downloadTextFile hands the text to an anchor download and releases the object URL', async () => {
  const createObjectURL = vi.fn<(blob: Blob) => string>(() => 'blob:kukuri/logs');
  const revokeObjectURL = vi.fn();
  vi.stubGlobal('URL', { ...URL, createObjectURL, revokeObjectURL });
  const click = vi.spyOn(HTMLAnchorElement.prototype, 'click').mockImplementation(function (this: HTMLAnchorElement) {
    expect(this.download).toBe('kukuri-logs.txt');
    expect(this.href).toBe('blob:kukuri/logs');
  });

  downloadTextFile('kukuri-logs.txt', 'hello');

  expect(click).toHaveBeenCalledTimes(1);
  const blob = createObjectURL.mock.calls[0]?.[0];
  expect(blob?.type).toBe('text/plain;charset=utf-8');
  expect(await blob?.text()).toBe('hello');
  expect(revokeObjectURL).toHaveBeenCalledWith('blob:kukuri/logs');
  vi.unstubAllGlobals();
});
