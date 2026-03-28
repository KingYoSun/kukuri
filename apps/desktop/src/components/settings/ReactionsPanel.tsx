import { ChangeEvent, useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';

import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Notice } from '@/components/ui/notice';
import { type CustomReactionCropRect } from '@/lib/api';

import { type ReactionsPanelView } from './types';

type ReactionsPanelProps = {
  view: ReactionsPanelView;
  creating: boolean;
  mediaObjectUrls?: Record<string, string | null>;
  onCreateAsset: (file: File, cropRect: CustomReactionCropRect) => void;
  onRemoveBookmark: (assetId: string) => void;
};

type CropDraft = {
  x: number;
  y: number;
  size: number;
  width: number;
  height: number;
};

function centeredCrop(width: number, height: number): CropDraft {
  const size = Math.min(width, height);
  return {
    x: Math.floor((width - size) / 2),
    y: Math.floor((height - size) / 2),
    size,
    width,
    height,
  };
}

function clampCrop(crop: CropDraft): CropDraft {
  const maxSize = Math.max(1, Math.min(crop.width, crop.height));
  const size = Math.min(Math.max(1, crop.size), maxSize);
  return {
    ...crop,
    size,
    x: Math.min(Math.max(0, crop.x), Math.max(0, crop.width - size)),
    y: Math.min(Math.max(0, crop.y), Math.max(0, crop.height - size)),
  };
}

export function ReactionsPanel({
  view,
  creating,
  mediaObjectUrls = {},
  onCreateAsset,
  onRemoveBookmark,
}: ReactionsPanelProps) {
  const { t } = useTranslation(['settings', 'common']);
  const [draftFile, setDraftFile] = useState<File | null>(null);
  const [draftPreviewUrl, setDraftPreviewUrl] = useState<string | null>(null);
  const [draftCrop, setDraftCrop] = useState<CropDraft | null>(null);
  const [draftError, setDraftError] = useState<string | null>(null);

  useEffect(() => {
    return () => {
      if (draftPreviewUrl) {
        URL.revokeObjectURL(draftPreviewUrl);
      }
    };
  }, [draftPreviewUrl]);

  const cropPreviewStyle = useMemo(() => {
    if (!draftPreviewUrl || !draftCrop) {
      return undefined;
    }
    const backgroundScale = 128 / draftCrop.size;
    return {
      backgroundImage: `url(${draftPreviewUrl})`,
      backgroundSize: `${draftCrop.width * backgroundScale}px ${draftCrop.height * backgroundScale}px`,
      backgroundPosition: `${-draftCrop.x * backgroundScale}px ${-draftCrop.y * backgroundScale}px`,
    };
  }, [draftCrop, draftPreviewUrl]);

  const handleDraftFileChange = async (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0] ?? null;
    if (!file) {
      return;
    }
    if (draftPreviewUrl) {
      URL.revokeObjectURL(draftPreviewUrl);
    }
    const nextUrl = URL.createObjectURL(file);
    setDraftPreviewUrl(nextUrl);
    setDraftFile(file);
    setDraftError(null);
    try {
      const dimensions = await new Promise<{ width: number; height: number }>((resolve, reject) => {
        const image = new Image();
        image.onload = () => resolve({ width: image.naturalWidth, height: image.naturalHeight });
        image.onerror = () => reject(new Error('failed to read image dimensions'));
        image.src = nextUrl;
      });
      setDraftCrop(centeredCrop(dimensions.width, dimensions.height));
    } catch (error) {
      setDraftCrop(null);
      setDraftError(error instanceof Error ? error.message : 'failed to read image dimensions');
    }
  };

  return (
    <div className='shell-main-stack reactions-panel'>
      <section className='shell-main-stack'>
        <div>
          <h4>{t('reactions.myCustomReactions')}</h4>
          <small>{t('reactions.myCustomReactionsHint')}</small>
        </div>
        <Label>
          <span>{t('reactions.uploadLabel')}</span>
          <Input type='file' accept='image/*,.gif' onChange={(event) => void handleDraftFileChange(event)} />
        </Label>
        {draftError ? <Notice tone='destructive'>{draftError}</Notice> : null}
        {draftFile && draftPreviewUrl && draftCrop ? (
          <div className='reactions-editor-grid'>
            <div className='reactions-editor-source'>
              <img src={draftPreviewUrl} alt={draftFile.name} className='reactions-editor-image' />
              <div className='topic-diagnostic topic-diagnostic-secondary'>
                <span>
                  {t('reactions.sourceSize', {
                    width: draftCrop.width,
                    height: draftCrop.height,
                  })}
                </span>
              </div>
            </div>
            <div className='shell-main-stack'>
              <div className='reactions-crop-fields'>
                <Label>
                  <span>X</span>
                  <Input
                    type='number'
                    min={0}
                    max={Math.max(0, draftCrop.width - draftCrop.size)}
                    value={draftCrop.x}
                    onChange={(event) =>
                      setDraftCrop((current) =>
                        current
                          ? clampCrop({ ...current, x: Number(event.target.value) || 0 })
                          : current
                      )
                    }
                  />
                </Label>
                <Label>
                  <span>Y</span>
                  <Input
                    type='number'
                    min={0}
                    max={Math.max(0, draftCrop.height - draftCrop.size)}
                    value={draftCrop.y}
                    onChange={(event) =>
                      setDraftCrop((current) =>
                        current
                          ? clampCrop({ ...current, y: Number(event.target.value) || 0 })
                          : current
                      )
                    }
                  />
                </Label>
                <Label>
                  <span>{t('reactions.cropSize')}</span>
                  <Input
                    type='number'
                    min={1}
                    max={Math.min(draftCrop.width, draftCrop.height)}
                    value={draftCrop.size}
                    onChange={(event) =>
                      setDraftCrop((current) =>
                        current
                          ? clampCrop({ ...current, size: Number(event.target.value) || 1 })
                          : current
                      )
                    }
                  />
                </Label>
              </div>
              <div className='reactions-preview-card'>
                <div className='reactions-preview-thumb' style={cropPreviewStyle} aria-label={t('reactions.preview')} />
                <small>{t('reactions.previewHint')}</small>
              </div>
              <div className='post-actions-inline'>
                <Button
                  type='button'
                  disabled={creating}
                  onClick={() =>
                    onCreateAsset(draftFile, {
                      x: draftCrop.x,
                      y: draftCrop.y,
                      size: draftCrop.size,
                    })
                  }
                >
                  {t('common:actions.save')}
                </Button>
                <Button
                  variant='secondary'
                  type='button'
                  disabled={creating}
                  onClick={() => {
                    if (draftPreviewUrl) {
                      URL.revokeObjectURL(draftPreviewUrl);
                    }
                    setDraftFile(null);
                    setDraftPreviewUrl(null);
                    setDraftCrop(null);
                    setDraftError(null);
                  }}
                >
                  {t('common:actions.cancel')}
                </Button>
              </div>
            </div>
          </div>
        ) : null}
        <div className='reactions-asset-grid'>
          {view.ownedAssets.map((asset) => (
            <div key={asset.asset_id} className='reactions-asset-card'>
              {typeof mediaObjectUrls[asset.blob_hash] === 'string' ? (
                <img
                  className='reactions-asset-thumb'
                  src={mediaObjectUrls[asset.blob_hash] ?? undefined}
                  alt={asset.asset_id}
                  data-asset-id={asset.asset_id}
                />
              ) : (
                <div className='reactions-asset-thumb reactions-asset-placeholder'>
                  {asset.asset_id.slice(0, 4)}
                </div>
              )}
              <small>{asset.mime}</small>
            </div>
          ))}
          {view.ownedAssets.length === 0 ? <p className='empty-state'>{t('reactions.noOwnedAssets')}</p> : null}
        </div>
      </section>

      <section className='shell-main-stack'>
        <div>
          <h4>{t('reactions.savedReactions')}</h4>
          <small>{t('reactions.savedReactionsHint')}</small>
        </div>
        <div className='reactions-saved-list'>
          {view.bookmarkedAssets.map((asset) => (
            <article key={asset.asset_id} className='post-card reactions-saved-card'>
              {typeof mediaObjectUrls[asset.blob_hash] === 'string' ? (
                <img
                  className='reactions-asset-thumb'
                  src={mediaObjectUrls[asset.blob_hash] ?? undefined}
                  alt={asset.asset_id}
                />
              ) : null}
              <div className='topic-diagnostic topic-diagnostic-secondary'>
                <span>{asset.owner_pubkey}</span>
                <span>{asset.mime}</span>
              </div>
              <div className='post-actions'>
                <Button variant='secondary' type='button' onClick={() => onRemoveBookmark(asset.asset_id)}>
                  {t('common:actions.clear')}
                </Button>
              </div>
            </article>
          ))}
          {view.bookmarkedAssets.length === 0 ? (
            <p className='empty-state'>{t('reactions.noSavedAssets')}</p>
          ) : null}
        </div>
      </section>

      {view.panelError ? <Notice tone='destructive'>{view.panelError}</Notice> : null}
    </div>
  );
}
