import { describe, expect, test } from 'vitest';

import type { AttachmentView, PostView } from '@/lib/api';

import { buildPostMediaView } from './postMediaView';

const IMAGE_HASH = 'a'.repeat(64);
const SECOND_IMAGE_HASH = 'b'.repeat(64);
const MANIFEST_HASH = 'c'.repeat(64);
const POSTER_HASH = 'd'.repeat(64);

function post(attachments: AttachmentView[]): Pick<PostView, 'object_id' | 'attachments'> {
  return { object_id: 'post-1', attachments };
}

function attachment(overrides: Partial<AttachmentView> & { hash: string }): AttachmentView {
  return {
    mime: 'image/png',
    bytes: 2048,
    role: 'image_original',
    status: 'Available',
    ...overrides,
  };
}

function build(
  attachments: AttachmentView[],
  overrides: {
    mediaObjectUrls?: Record<string, string | null>;
    adultContentGated?: boolean;
    unsupportedVideoManifests?: Record<string, true>;
  } = {}
) {
  return buildPostMediaView(post(attachments), {
    mediaObjectUrls: overrides.mediaObjectUrls ?? {},
    adultContentGated: overrides.adultContentGated ?? false,
    unsupportedVideoManifests: overrides.unsupportedVideoManifests ?? {},
    locale: 'en',
  });
}

describe('buildPostMediaView', () => {
  test('reports no media kind for a post without attachments', () => {
    expect(build([])).toMatchObject({
      objectId: 'post-1',
      kind: null,
      extraAttachmentCount: 0,
      state: 'ready',
    });
  });

  test('builds an image gallery with the primary image first and counts the rest', () => {
    const media = build(
      [attachment({ hash: IMAGE_HASH }), attachment({ hash: SECOND_IMAGE_HASH, bytes: 4096 })],
      {
        mediaObjectUrls: { [IMAGE_HASH]: 'blob:first', [SECOND_IMAGE_HASH]: 'blob:second' },
      }
    );

    expect(media).toMatchObject({
      kind: 'image',
      state: 'ready',
      extraAttachmentCount: 1,
      imagePreviewSrc: 'blob:first',
      metaMime: 'image/png',
      currentImageIndex: 0,
    });
    expect(media.imageGalleryItems?.map((item) => item.hash)).toEqual([
      IMAGE_HASH,
      SECOND_IMAGE_HASH,
    ]);
  });

  test('stays loading until the object url settles and reports unavailable when it settles empty', () => {
    expect(build([attachment({ hash: IMAGE_HASH })])).toMatchObject({
      kind: 'image',
      state: 'loading',
      imagePreviewSrc: null,
    });
    expect(
      build([attachment({ hash: IMAGE_HASH })], { mediaObjectUrls: { [IMAGE_HASH]: null } })
    ).toMatchObject({ kind: 'image', state: 'unavailable' });
  });

  test('gates every preview source while adult display is off', () => {
    const media = build([attachment({ hash: IMAGE_HASH })], {
      adultContentGated: true,
      mediaObjectUrls: { [IMAGE_HASH]: 'blob:first' },
    });

    expect(media).toMatchObject({ kind: 'image', state: 'gated', imagePreviewSrc: null });
    expect(media.imageGalleryItems).toEqual([]);
  });

  test('prefers the video manifest for playback, meta and report identity', () => {
    const attachments = [
      attachment({ hash: MANIFEST_HASH, mime: 'video/mp4', role: 'video_manifest', bytes: 8192 }),
      attachment({ hash: POSTER_HASH, mime: 'image/jpeg', role: 'video_poster', bytes: 1024 }),
    ];

    expect(
      build(attachments, { mediaObjectUrls: { [MANIFEST_HASH]: 'blob:video' } })
    ).toMatchObject({
      kind: 'video',
      state: 'ready',
      videoPlaybackSrc: 'blob:video',
      videoReportHash: MANIFEST_HASH,
      metaMime: 'video/mp4',
      extraAttachmentCount: 0,
    });
    expect(
      build(attachments, {
        mediaObjectUrls: { [MANIFEST_HASH]: 'blob:video' },
        unsupportedVideoManifests: { [MANIFEST_HASH]: true },
      }).videoUnsupportedOnClient
    ).toBe(true);
  });

  test('keeps a video poster out of the image gallery', () => {
    const media = build(
      [
        attachment({ hash: MANIFEST_HASH, mime: 'video/mp4', role: 'video_manifest' }),
        attachment({ hash: POSTER_HASH, mime: 'image/jpeg', role: 'video_poster' }),
      ],
      { mediaObjectUrls: { [POSTER_HASH]: 'blob:poster' } }
    );

    expect(media.imageGalleryItems).toEqual([]);
    expect(media).toMatchObject({ kind: 'video', state: 'ready', videoPosterPreviewSrc: 'blob:poster' });
  });
});
