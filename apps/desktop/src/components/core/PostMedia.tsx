import type * as React from 'react';
import { useTranslation } from 'react-i18next';
import { Flag } from 'lucide-react';

import { IconButton } from '@/components/ui/icon-button';

import { type PostMediaView } from './types';

type PostMediaProps = {
  media: PostMediaView;
  showUnavailableDiagnostic?: boolean;
  onOpenImage?: (index: number) => void;
  /// 動画添付そのものを media として通報する(#697)。未指定なら操作を出さない。
  onReportVideo?: (hash: string) => void;
};

export function PostMedia({
  media,
  showUnavailableDiagnostic = false,
  onOpenImage,
  onReportVideo,
}: PostMediaProps) {
  const { t } = useTranslation(['common', 'shell']);
  const videoReportHash = media.kind === 'video' ? media.videoReportHash : null;

  if (!media.kind) {
    return null;
  }
  if (media.state === 'unavailable') {
    return showUnavailableDiagnostic ? (
      <p className='topic-diagnostic topic-diagnostic-secondary' role='status'>
        {t('media.unavailable')}
      </p>
    ) : null;
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
        {onReportVideo && videoReportHash ? (
          <IconButton
            variant='secondary'
            type='button'
            className='media-video-report'
            onClick={() => onReportVideo(videoReportHash)}
            label={t('media.reportVideo')}
            data-testid={`media-video-report-${media.objectId}`}
          >
            <Flag className='size-4' aria-hidden='true' />
          </IconButton>
        ) : null}

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
