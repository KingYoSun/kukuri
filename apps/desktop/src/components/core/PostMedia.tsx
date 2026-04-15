import type * as React from 'react';
import { useTranslation } from 'react-i18next';

import { type PostMediaView } from './types';

type PostMediaProps = {
  media: PostMediaView;
  onOpenImage?: (index: number) => void;
};

export function PostMedia({ media, onOpenImage }: PostMediaProps) {
  const { t } = useTranslation('common');

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
          {media.kind === 'video' ? <span className='media-type-badge'>{t('media.video')}</span> : null}
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
            alt={t('media.videoPosterAlt')}
            data-testid={`media-preview-${media.objectId}`}
          />
        ) : media.kind === 'image' && media.imagePreviewSrc ? (
          <button
            className='media-image-trigger'
            type='button'
            onClick={() => onOpenImage?.(media.currentImageIndex ?? 0)}
            aria-label={t('media.imageAlt')}
          >
            <img
              className='media-preview'
              src={media.imagePreviewSrc}
              alt={t('media.imageAlt')}
              data-testid={`media-preview-${media.objectId}`}
            />
          </button>
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
