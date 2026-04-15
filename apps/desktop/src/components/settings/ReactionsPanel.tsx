import { ChangeEvent, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';

import { Button } from '@/components/ui/button';
import { ImageCropDialog } from '@/components/ui/ImageCropDialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Notice } from '@/components/ui/notice';
import { type CustomReactionCropRect } from '@/lib/api';

import { type ReactionsPanelView } from './types';

type ReactionsPanelProps = {
  view: ReactionsPanelView;
  creating: boolean;
  mediaObjectUrls?: Record<string, string | null>;
  onCreateAsset: (file: File, cropRect: CustomReactionCropRect, searchKey: string) => void;
  onRemoveBookmark: (assetId: string) => void;
};

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
  const [draftCrop, setDraftCrop] = useState<CustomReactionCropRect | null>(null);
  const [draftSearchKey, setDraftSearchKey] = useState('');
  const [draftError, setDraftError] = useState<string | null>(null);
  const [cropDialogOpen, setCropDialogOpen] = useState(false);
  const [cropDialogFile, setCropDialogFile] = useState<File | null>(null);

  useEffect(() => {
    return () => {
      if (draftPreviewUrl) {
        URL.revokeObjectURL(draftPreviewUrl);
      }
    };
  }, [draftPreviewUrl]);

  const handleDraftFileChange = (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0] ?? null;
    if (!file) {
      return;
    }
    setCropDialogFile(file);
    setCropDialogOpen(true);
    setDraftError(null);
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
          <Input type='file' accept='image/*,.gif' onChange={handleDraftFileChange} />
        </Label>
        {draftError ? <Notice tone='destructive'>{draftError}</Notice> : null}
        {draftFile && draftPreviewUrl && draftCrop ? (
          <div className='reactions-editor-grid'>
            <div className='shell-main-stack'>
              <div className='reactions-preview-card'>
                <div
                  className='reactions-preview-thumb'
                  style={{ backgroundImage: `url(${draftPreviewUrl})` }}
                  aria-label={t('reactions.preview')}
                />
                <small>{t('reactions.previewHint')}</small>
              </div>
              <Button
                variant='secondary'
                type='button'
                onClick={() => {
                  setCropDialogFile(draftFile);
                  setCropDialogOpen(true);
                }}
              >
                {t('reactions.editCrop', { defaultValue: 'Edit crop' })}
              </Button>
            </div>
            <div className='shell-main-stack'>
              <Label>
                <span>{t('reactions.searchKeyLabel')}</span>
                <Input
                  value={draftSearchKey}
                  placeholder={t('reactions.searchKeyPlaceholder')}
                  onChange={(event) => {
                    setDraftSearchKey(event.target.value);
                    if (draftError === t('reactions.searchKeyRequired')) {
                      setDraftError(null);
                    }
                  }}
                />
              </Label>
              <div className='post-actions-inline'>
                <Button
                  type='button'
                  disabled={creating}
                  onClick={() => {
                    const normalizedSearchKey = draftSearchKey.trim();
                    if (!normalizedSearchKey) {
                      setDraftError(t('reactions.searchKeyRequired'));
                      return;
                    }
                    onCreateAsset(draftFile, draftCrop, normalizedSearchKey);
                  }}
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
                    setDraftSearchKey('');
                    setDraftError(null);
                  }}
                >
                  {t('common:actions.cancel', { defaultValue: 'Cancel' })}
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
              <strong>{asset.search_key}</strong>
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
                <span>{asset.search_key}</span>
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

      <ImageCropDialog
        open={cropDialogOpen}
        file={cropDialogFile}
        title={t('reactions.cropTitle', { defaultValue: 'Crop reaction image' })}
        description={t('reactions.cropDescription', {
          defaultValue: 'Drag to reposition and use zoom to choose the visible square.',
        })}
        confirmLabel={t('common:actions.save')}
        onOpenChange={(open) => {
          setCropDialogOpen(open);
          if (!open) {
            setCropDialogFile(null);
          }
        }}
        onConfirm={async ({ file, cropRect, croppedFile }) => {
          if (draftPreviewUrl) {
            URL.revokeObjectURL(draftPreviewUrl);
          }
          setDraftFile(file);
          setDraftCrop(cropRect);
          setDraftPreviewUrl(URL.createObjectURL(croppedFile));
          setCropDialogOpen(false);
          setCropDialogFile(null);
        }}
      />
    </div>
  );
}
