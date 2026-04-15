import { useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';

import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogBody,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';

import type { CustomReactionCropRect } from '@/lib/api';

type ImageCropDialogProps = {
  open: boolean;
  file: File | null;
  title: string;
  description: string;
  confirmLabel: string;
  onOpenChange: (open: boolean) => void;
  onConfirm: (result: {
    file: File;
    cropRect: CustomReactionCropRect;
    croppedFile: File;
    previewUrl: string;
  }) => void | Promise<void>;
};

type ImageDimensions = {
  width: number;
  height: number;
};

const VIEWPORT_SIZE = 240;

function centeredCrop(dimensions: ImageDimensions, zoom: number): CustomReactionCropRect {
  const size = Math.max(1, Math.floor(Math.min(dimensions.width, dimensions.height) / zoom));
  return {
    x: Math.max(0, Math.floor((dimensions.width - size) / 2)),
    y: Math.max(0, Math.floor((dimensions.height - size) / 2)),
    size,
  };
}

function clampCrop(
  cropRect: CustomReactionCropRect,
  dimensions: ImageDimensions
): CustomReactionCropRect {
  const maxSize = Math.max(1, Math.min(dimensions.width, dimensions.height));
  const size = Math.min(Math.max(1, Math.round(cropRect.size)), maxSize);
  return {
    size,
    x: Math.min(Math.max(0, Math.round(cropRect.x)), Math.max(0, dimensions.width - size)),
    y: Math.min(Math.max(0, Math.round(cropRect.y)), Math.max(0, dimensions.height - size)),
  };
}

async function loadImageDimensions(previewUrl: string) {
  return await new Promise<ImageDimensions>((resolve, reject) => {
    const image = new Image();
    image.onload = () =>
      resolve({
        width: image.naturalWidth,
        height: image.naturalHeight,
      });
    image.onerror = () => reject(new Error('failed to read image dimensions'));
    image.src = previewUrl;
  });
}

async function renderCroppedImage(
  previewUrl: string,
  file: File,
  cropRect: CustomReactionCropRect
) {
  const image = await new Promise<HTMLImageElement>((resolve, reject) => {
    const element = new Image();
    element.onload = () => resolve(element);
    element.onerror = () => reject(new Error('failed to render crop preview'));
    element.src = previewUrl;
  });
  const canvas = document.createElement('canvas');
  const outputSize = 256;
  canvas.width = outputSize;
  canvas.height = outputSize;
  const context = canvas.getContext('2d');
  if (!context) {
    throw new Error('failed to prepare crop canvas');
  }
  context.drawImage(
    image,
    cropRect.x,
    cropRect.y,
    cropRect.size,
    cropRect.size,
    0,
    0,
    outputSize,
    outputSize
  );
  const blob = await new Promise<Blob>((resolve, reject) => {
    canvas.toBlob((value) => {
      if (value) {
        resolve(value);
        return;
      }
      reject(new Error('failed to export crop image'));
    }, file.type || 'image/png');
  });
  return new File([blob], file.name, { type: blob.type || file.type || 'image/png' });
}

export function ImageCropDialog({
  open,
  file,
  title,
  description,
  confirmLabel,
  onOpenChange,
  onConfirm,
}: ImageCropDialogProps) {
  const { t } = useTranslation('common');
  const [previewUrl, setPreviewUrl] = useState<string | null>(null);
  const [dimensions, setDimensions] = useState<ImageDimensions | null>(null);
  const [zoom, setZoom] = useState(1);
  const [cropRect, setCropRect] = useState<CustomReactionCropRect | null>(null);
  const [error, setError] = useState<string | null>(null);
  const dragStateRef = useRef<{ x: number; y: number } | null>(null);

  useEffect(() => {
    if (!file || !open) {
      return;
    }
    const nextPreviewUrl = URL.createObjectURL(file);
    setPreviewUrl(nextPreviewUrl);
    setError(null);
    void loadImageDimensions(nextPreviewUrl)
      .then((nextDimensions) => {
        setDimensions(nextDimensions);
        setZoom(1);
        setCropRect(centeredCrop(nextDimensions, 1));
      })
      .catch((nextError) => {
        setError(nextError instanceof Error ? nextError.message : 'failed to read image');
      });
    return () => {
      URL.revokeObjectURL(nextPreviewUrl);
    };
  }, [file, open]);

  const scale = useMemo(() => {
    if (!cropRect) {
      return 1;
    }
    return VIEWPORT_SIZE / cropRect.size;
  }, [cropRect]);

  const previewStyle = useMemo(() => {
    if (!previewUrl || !cropRect || !dimensions) {
      return undefined;
    }
    const translateX = VIEWPORT_SIZE / 2 - (cropRect.x + cropRect.size / 2) * scale;
    const translateY = VIEWPORT_SIZE / 2 - (cropRect.y + cropRect.size / 2) * scale;
    return {
      backgroundImage: `url(${previewUrl})`,
      backgroundSize: `${dimensions.width * scale}px ${dimensions.height * scale}px`,
      backgroundPosition: `${translateX}px ${translateY}px`,
    };
  }, [cropRect, dimensions, previewUrl, scale]);

  const commitZoom = (nextZoom: number) => {
    if (!dimensions || !cropRect) {
      return;
    }
    const currentCenterX = cropRect.x + cropRect.size / 2;
    const currentCenterY = cropRect.y + cropRect.size / 2;
    const size = Math.max(1, Math.floor(Math.min(dimensions.width, dimensions.height) / nextZoom));
    setZoom(nextZoom);
    setCropRect(
      clampCrop(
        {
          x: currentCenterX - size / 2,
          y: currentCenterY - size / 2,
          size,
        },
        dimensions
      )
    );
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className='image-crop-dialog'>
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
          <DialogDescription>{description}</DialogDescription>
        </DialogHeader>
        <DialogBody className='image-crop-dialog-body'>
          {error ? <p className='error error-inline'>{error}</p> : null}
          {previewStyle ? (
            <div className='image-crop-layout'>
              <div
                className='image-crop-stage'
                onPointerDown={(event) => {
                  dragStateRef.current = { x: event.clientX, y: event.clientY };
                }}
                onPointerMove={(event) => {
                  if (!dragStateRef.current || !dimensions || !cropRect) {
                    return;
                  }
                  const deltaX = event.clientX - dragStateRef.current.x;
                  const deltaY = event.clientY - dragStateRef.current.y;
                  dragStateRef.current = { x: event.clientX, y: event.clientY };
                  setCropRect(
                    clampCrop(
                      {
                        ...cropRect,
                        x: cropRect.x - deltaX / scale,
                        y: cropRect.y - deltaY / scale,
                      },
                      dimensions
                    )
                  );
                }}
                onPointerUp={() => {
                  dragStateRef.current = null;
                }}
                onPointerLeave={() => {
                  dragStateRef.current = null;
                }}
                onWheel={(event) => {
                  if (!dimensions || !cropRect) {
                    return;
                  }
                  event.preventDefault();
                  const nextZoom = Math.min(4, Math.max(1, zoom + (event.deltaY < 0 ? 0.15 : -0.15)));
                  commitZoom(Number(nextZoom.toFixed(2)));
                }}
              >
                <div className='image-crop-preview' style={previewStyle} />
                <div className='image-crop-frame' aria-hidden='true' />
              </div>
              <div className='shell-main-stack'>
                <label className='shell-main-stack'>
                  <span>{t('actions.zoom', { defaultValue: 'Zoom' })}</span>
                  <Input
                    type='range'
                    min='1'
                    max='4'
                    step='0.05'
                    value={zoom}
                    onChange={(event) => commitZoom(Number(event.target.value))}
                  />
                </label>
              </div>
            </div>
          ) : null}
        </DialogBody>
        <DialogFooter>
          <Button variant='secondary' type='button' onClick={() => onOpenChange(false)}>
            {t('actions.cancel', { defaultValue: 'Cancel' })}
          </Button>
          <Button
            type='button'
            disabled={!file || !previewUrl || !cropRect}
            onClick={async () => {
              if (!file || !previewUrl || !cropRect) {
                return;
              }
              const croppedFile = await renderCroppedImage(previewUrl, file, cropRect);
              await onConfirm({
                file,
                cropRect,
                croppedFile,
                previewUrl,
              });
            }}
          >
            {confirmLabel}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
