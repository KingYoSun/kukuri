import type * as React from 'react';

import { type PostMediaView } from './types';

type PostMediaProps = {
  media: PostMediaView;
};

export function PostMedia({ media }: PostMediaProps) {
  if (!media.kind) {
    return null;
  }

  return (
    <>
      <div
        className={
          media.state === 'loading' ? 'media-frame media-frame-loading' : 'media-frame media-frame-ready'
        }
      >
        <div className='media-badges'>
          {media.statusLabel ? <span className='media-status-badge'>{media.statusLabel}</span> : null}
          {media.kind === 'video' ? <span className='media-type-badge'>video</span> : null}
          {media.extraAttachmentCount > 0 ? (
            <span className='media-count-badge'>+{media.extraAttachmentCount}</span>
          ) : null}
        </div>

        {media.kind === 'video' && media.videoPlaybackSrc && !media.videoUnsupportedOnClient ? (
          <video
            className='media-video'
            controls
            src={media.videoPlaybackSrc}
            preload='metadata'
            poster={media.videoPosterPreviewSrc ?? undefined}
            data-testid={`media-video-${media.objectId}`}
            {...media.videoProps}
          />
        ) : media.kind === 'video' && media.videoPosterPreviewSrc ? (
          <img
            className='media-preview'
            src={media.videoPosterPreviewSrc}
            alt='video poster'
            data-testid={`media-preview-${media.objectId}`}
          />
        ) : media.kind === 'image' && media.imagePreviewSrc ? (
          <img
            className='media-preview'
            src={media.imagePreviewSrc}
            alt='image attachment'
            data-testid={`media-preview-${media.objectId}`}
          />
        ) : (
          <div
            className='media-skeleton'
            data-testid={`media-skeleton-${media.objectId}`}
            aria-hidden='true'
          />
        )}
      </div>

      {media.metaMime || media.metaBytesLabel ? (
        <div className='media-meta'>
          <span>{media.metaMime}</span>
          <span>{media.metaBytesLabel}</span>
        </div>
      ) : null}
    </>
  );
}
