import { ChangeEvent, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';

import { Button } from '@/components/ui/button';
import {
  ContextActionMenu,
  type ContextActionMenuPosition,
} from '@/components/ui/context-action-menu';
import { ImageCropDialog } from '@/components/ui/ImageCropDialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Notice } from '@/components/ui/notice';
import { type CustomReactionCropRect } from '@/lib/api';
import { copyTextToClipboard } from '@/lib/utils';

import { type ReactionsPanelView } from './types';

type ReactionsPanelProps = {
  view: ReactionsPanelView;
  creating: boolean;
  mediaObjectUrls?: Record<string, string | null>;
  onCreateAsset: (file: File, cropRect: CustomReactionCropRect, searchKey: string) => void;
  onRemoveBookmark: (assetId: string) => Promise<void>;
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
  const [selectedAssetIds, setSelectedAssetIds] = useState<Set<string>>(new Set());
  const [removingBookmarks, setRemovingBookmarks] = useState(false);
  const [savedMenuPosition, setSavedMenuPosition] = useState<ContextActionMenuPosition | null>(
    null
  );
  const [savedMenuAssetId, setSavedMenuAssetId] = useState<string | null>(null);

  useEffect(() => {
    return () => {
      if (draftPreviewUrl) {
        URL.revokeObjectURL(draftPreviewUrl);
      }
    };
  }, [draftPreviewUrl]);

  useEffect(() => {
    const validAssetIds = new Set(view.bookmarkedAssets.map((asset) => asset.asset_id));
    setSelectedAssetIds((current) => {
      const next = new Set([...current].filter((assetId) => validAssetIds.has(assetId)));
      return next.size === current.size ? current : next;
    });
    if (savedMenuAssetId && !validAssetIds.has(savedMenuAssetId)) {
      setSavedMenuAssetId(null);
      setSavedMenuPosition(null);
    }
  }, [savedMenuAssetId, view.bookmarkedAssets]);

  const handleDraftFileChange = (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0] ?? null;
    if (!file) {
      return;
    }
    setCropDialogFile(file);
    setCropDialogOpen(true);
    setDraftError(null);
  };

  const selectedSavedCount = selectedAssetIds.size;
  const allSavedSelected =
    view.bookmarkedAssets.length > 0 &&
    view.bookmarkedAssets.every((asset) => selectedAssetIds.has(asset.asset_id));
  const savedMenuAsset =
    view.bookmarkedAssets.find((asset) => asset.asset_id === savedMenuAssetId) ?? null;

  const handleToggleSavedAsset = (assetId: string, checked: boolean) => {
    setSelectedAssetIds((current) => {
      const next = new Set(current);
      if (checked) {
        next.add(assetId);
      } else {
        next.delete(assetId);
      }
      return next;
    });
  };

  const handleRemoveSavedAssets = async (assetIds: string[]) => {
    if (assetIds.length === 0) {
      return;
    }
    setRemovingBookmarks(true);
    try {
      await Promise.all(assetIds.map((assetId) => onRemoveBookmark(assetId)));
    } finally {
      setRemovingBookmarks(false);
    }
  };

  const savedMenuItems = savedMenuAsset
    ? [
        {
          id: 'copy-hash',
          label: t('common:actions.copyHash'),
          onSelect: async () => {
            await copyTextToClipboard(savedMenuAsset.blob_hash);
          },
        },
        {
          id: 'clear',
          label: t('common:actions.clear'),
          tone: 'danger' as const,
          onSelect: async () => {
            await handleRemoveSavedAssets([savedMenuAsset.asset_id]);
          },
        },
      ]
    : [];

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
        <div className='reactions-saved-header'>
          <div>
            <h4>{t('reactions.savedReactions')}</h4>
            <small>{t('reactions.savedReactionsHint')}</small>
          </div>
          {view.bookmarkedAssets.length > 0 ? (
            <div className='reactions-saved-toolbar'>
              <label className='reactions-saved-select-all'>
                <input
                  type='checkbox'
                  checked={allSavedSelected}
                  onChange={(event) => {
                    if (event.target.checked) {
                      setSelectedAssetIds(
                        new Set(view.bookmarkedAssets.map((asset) => asset.asset_id))
                      );
                    } else {
                      setSelectedAssetIds(new Set());
                    }
                  }}
                />
                <span>{t('reactions.selectAll')}</span>
              </label>
              <Button
                variant='secondary'
                type='button'
                disabled={selectedSavedCount === 0 || removingBookmarks}
                onClick={() => void handleRemoveSavedAssets([...selectedAssetIds])}
              >
                {t('reactions.clearSelected')}
              </Button>
            </div>
          ) : null}
        </div>
        <div className='reactions-saved-list'>
          {view.bookmarkedAssets.map((asset) => {
            const previewUrl =
              typeof mediaObjectUrls[asset.blob_hash] === 'string'
                ? mediaObjectUrls[asset.blob_hash]
                : null;
            const isSelected = selectedAssetIds.has(asset.asset_id);
            return (
              <article
                key={asset.asset_id}
                className={`reactions-saved-tile${isSelected ? ' reactions-saved-tile-selected' : ''}`}
                aria-label={asset.search_key}
                onContextMenu={(event) => {
                  event.preventDefault();
                  setSavedMenuAssetId(asset.asset_id);
                  setSavedMenuPosition({
                    x: event.clientX,
                    y: event.clientY,
                  });
                }}
              >
                <label className='reactions-saved-checkbox'>
                  <input
                    type='checkbox'
                    aria-label={t('reactions.selectReaction', { key: asset.search_key })}
                    checked={isSelected}
                    onChange={(event) =>
                      handleToggleSavedAsset(asset.asset_id, event.target.checked)
                    }
                  />
                </label>
                {previewUrl ? (
                  <img className='reactions-asset-thumb' src={previewUrl} alt={asset.search_key} />
                ) : (
                  <div className='reactions-asset-thumb reactions-asset-placeholder' aria-hidden='true'>
                    {asset.search_key.slice(0, 2)}
                  </div>
                )}
              </article>
            );
          })}
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
      <ContextActionMenu
        open={savedMenuAsset !== null}
        position={savedMenuPosition}
        items={savedMenuItems}
        onClose={() => {
          setSavedMenuAssetId(null);
          setSavedMenuPosition(null);
        }}
      />
    </div>
  );
}
