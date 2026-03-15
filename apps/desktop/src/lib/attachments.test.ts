import { afterEach, describe, expect, it, vi } from 'vitest';

import { blobToBase64, blobToCreateAttachment } from './attachments';

describe('attachment encoding', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('encodes blobs without relying on window.btoa', async () => {
    const btoaSpy = vi.fn(() => {
      throw new Error('btoa should not be called');
    });
    vi.stubGlobal('btoa', btoaSpy);

    await expect(blobToBase64(new Blob(['hello world'], { type: 'text/plain' }))).resolves.toBe(
      'aGVsbG8gd29ybGQ='
    );
    expect(btoaSpy).not.toHaveBeenCalled();
  });

  it('preserves attachment metadata when converting from blobs', async () => {
    await expect(
      blobToCreateAttachment(
        new Blob(['image-bytes'], { type: 'image/png' }),
        'reply.png',
        'image_original'
      )
    ).resolves.toEqual({
      file_name: 'reply.png',
      mime: 'image/png',
      byte_size: 11,
      data_base64: 'aW1hZ2UtYnl0ZXM=',
      role: 'image_original',
    });
  });
});
