import { afterEach, describe, expect, it, vi } from 'vitest';

import {
  blobToBase64,
  blobToCreateAttachment,
  clipboardImageFiles,
} from './attachments';

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

describe('clipboard image files (#1172)', () => {
  it('keeps clipboard image order, ignores non-images, and prefers file items', () => {
    const first = new File(['first'], 'first.png', { type: 'image/png' });
    const second = new File(['second'], 'second.webp', { type: 'image/webp' });
    const ignored = new File(['notes'], 'notes.txt', { type: 'text/plain' });

    expect(
      clipboardImageFiles({
        items: [
          { kind: 'string', type: 'text/plain', getAsFile: () => null },
          { kind: 'file', type: first.type, getAsFile: () => first },
          { kind: 'file', type: ignored.type, getAsFile: () => ignored },
          { kind: 'file', type: second.type, getAsFile: () => second },
        ],
        files: [new File(['fallback'], 'fallback.png', { type: 'image/png' })],
      })
    ).toEqual([first, second]);
  });

  it('falls back to clipboard files and gives unnamed images a useful name', () => {
    const unnamed = new File(['jpeg'], '', { type: 'image/jpeg', lastModified: 42 });
    const [image] = clipboardImageFiles({ items: [], files: [unnamed] });

    expect(image).toMatchObject({
      name: 'clipboard-image.jpg',
      type: 'image/jpeg',
      size: unnamed.size,
      lastModified: 42,
    });
  });
});
