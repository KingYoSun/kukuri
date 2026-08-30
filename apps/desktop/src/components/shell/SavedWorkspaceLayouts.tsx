import { LayoutPanelTop, Pencil, Save, Trash2 } from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';

import { Button } from '@/components/ui/button';
import { IconButton } from '@/components/ui/icon-button';
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
import { Notice } from '@/components/ui/notice';
import {
  captureSavedWorkspaceLayout,
  deleteSavedWorkspaceLayout,
  isSavedWorkspaceLayoutDirty,
  renameSavedWorkspaceLayout,
  savedWorkspaceLayoutNameError,
  updateSavedWorkspaceLayout,
  writeSavedWorkspaceLayouts,
  type SavedWorkspaceLayout,
  type SavedWorkspaceLayoutNameError,
} from '@/shell/savedWorkspaceLayouts';
import { useDesktopShellFieldSetter, useDesktopShellStore } from '@/shell/store';

type SavedWorkspaceLayoutsProps = {
  onActivateLayout: (layout: SavedWorkspaceLayout) => void;
};

type Confirmation =
  | { kind: 'activate'; layout: SavedWorkspaceLayout }
  | { kind: 'delete'; layout: SavedWorkspaceLayout }
  | null;

function newLayoutId() {
  if (typeof globalThis.crypto?.randomUUID === 'function') {
    return globalThis.crypto.randomUUID();
  }
  return `layout-${Date.now()}`;
}

export function SavedWorkspaceLayouts({
  onActivateLayout,
}: SavedWorkspaceLayoutsProps) {
  const { t } = useTranslation(['shell']);
  const workspaceState = useDesktopShellStore((state) => state.workspaceState);
  const layouts = useDesktopShellStore((state) => state.savedWorkspaceLayouts);
  const setWorkspaceState = useDesktopShellFieldSetter('workspaceState');
  const setLayouts = useDesktopShellFieldSetter('savedWorkspaceLayouts');
  const [name, setName] = useState('');
  const [renameId, setRenameId] = useState<string | null>(null);
  const [renameName, setRenameName] = useState('');
  const [nameError, setNameError] = useState<SavedWorkspaceLayoutNameError | null>(null);
  const [storageError, setStorageError] = useState(false);
  const [confirmation, setConfirmation] = useState<Confirmation>(null);
  const activeLayout = layouts.find((layout) => layout.id === workspaceState.activeLayoutId);
  const activeDirty = activeLayout
    ? isSavedWorkspaceLayoutDirty(workspaceState, activeLayout)
    : false;

  const persist = (next: SavedWorkspaceLayout[]) => {
    if (!writeSavedWorkspaceLayouts(window.localStorage, next)) {
      setStorageError(true);
      return false;
    }
    setStorageError(false);
    setLayouts(next);
    return true;
  };

  const saveNew = () => {
    const error = savedWorkspaceLayoutNameError(layouts, name);
    setNameError(error);
    if (error) return;
    const layout = captureSavedWorkspaceLayout(newLayoutId(), name, workspaceState);
    if (!persist([...layouts, layout])) return;
    setWorkspaceState((current) => ({ ...current, activeLayoutId: layout.id }));
    setName('');
  };

  const saveActive = () => {
    if (!activeLayout) return;
    const updated = updateSavedWorkspaceLayout(activeLayout, workspaceState);
    persist(layouts.map((layout) => (layout.id === updated.id ? updated : layout)));
  };

  const beginRename = (layout: SavedWorkspaceLayout) => {
    setRenameId(layout.id);
    setRenameName(layout.name);
    setNameError(null);
  };

  const commitRename = () => {
    if (!renameId) return;
    const error = savedWorkspaceLayoutNameError(layouts, renameName, renameId);
    setNameError(error);
    if (error) return;
    if (!persist(renameSavedWorkspaceLayout(layouts, renameId, renameName))) return;
    setRenameId(null);
    setRenameName('');
  };

  const activate = (layout: SavedWorkspaceLayout) => {
    const hasUnsavedWorkspace = activeLayout
      ? activeDirty
      : isSavedWorkspaceLayoutDirty(workspaceState, layout);
    if (hasUnsavedWorkspace) {
      setConfirmation({ kind: 'activate', layout });
      return;
    }
    onActivateLayout(layout);
  };

  const confirmAction = () => {
    if (!confirmation) return;
    if (confirmation.kind === 'activate') {
      onActivateLayout(confirmation.layout);
    } else {
      const next = deleteSavedWorkspaceLayout(layouts, confirmation.layout.id);
      if (persist(next) && workspaceState.activeLayoutId === confirmation.layout.id) {
        setWorkspaceState((current) => ({ ...current, activeLayoutId: null }));
      }
    }
    setConfirmation(null);
  };

  const nameErrorMessage = nameError
    ? t(`shell:controlCenter.layouts.errors.${nameError}`)
    : null;

  return (
    <div className='space-y-3 rounded-[16px] border border-[var(--border-subtle)] p-3'>
      <div className='flex items-center gap-2'>
        <LayoutPanelTop className='size-4' aria-hidden='true' />
        <h4 className='text-sm font-semibold'>{t('shell:controlCenter.layouts.title')}</h4>
      </div>
      <div className='flex min-w-0 flex-wrap gap-2'>
        <Input
          className='min-w-[12rem] flex-1'
          value={name}
          aria-label={t('shell:controlCenter.layouts.nameLabel')}
          placeholder={t('shell:controlCenter.layouts.namePlaceholder')}
          onChange={(event) => {
            setName(event.target.value);
            setNameError(null);
          }}
        />
        <Button variant='secondary' className='min-h-11' onClick={saveNew}>
          <Save className='size-4' aria-hidden='true' />
          {t('shell:controlCenter.layouts.saveNew')}
        </Button>
      </div>
      {nameErrorMessage ? <Notice tone='warning'>{nameErrorMessage}</Notice> : null}
      {storageError ? (
        <Notice tone='destructive'>{t('shell:controlCenter.layouts.errors.storage')}</Notice>
      ) : null}
      {layouts.length === 0 ? (
        <p className='text-sm text-[var(--muted-foreground)]'>
          {t('shell:controlCenter.layouts.empty')}
        </p>
      ) : (
        <ul className='space-y-2'>
          {layouts.map((layout) => {
            const active = layout.id === workspaceState.activeLayoutId;
            const dirty = active && isSavedWorkspaceLayoutDirty(workspaceState, layout);
            return (
              <li key={layout.id} className='rounded-[14px] bg-[var(--surface-panel-soft)] p-2'>
                {renameId === layout.id ? (
                  <div className='flex min-w-0 flex-wrap gap-2'>
                    <Input
                      className='min-w-[12rem] flex-1'
                      value={renameName}
                      aria-label={t('shell:controlCenter.layouts.renameLabel')}
                      onChange={(event) => {
                        setRenameName(event.target.value);
                        setNameError(null);
                      }}
                    />
                    <Button variant='secondary' className='min-h-11' onClick={commitRename}>
                      {t('shell:controlCenter.layouts.saveName')}
                    </Button>
                    <Button
                      variant='ghost'
                      className='min-h-11'
                      onClick={() => setRenameId(null)}
                    >
                      {t('shell:controlCenter.layouts.cancel')}
                    </Button>
                  </div>
                ) : (
                  <div className='flex min-w-0 flex-wrap items-center gap-2'>
                    <Button
                      variant={active ? 'primary' : 'secondary'}
                      className='min-h-11 min-w-[10rem] flex-1 justify-start'
                      aria-current={active ? 'true' : undefined}
                      onClick={() => activate(layout)}
                    >
                      <span className='truncate'>{layout.name}</span>
                    </Button>
                    {dirty ? (
                      <Button
                        variant='secondary'
                        className='min-h-11'
                        aria-label={t('shell:controlCenter.layouts.saveChangesTo', {
                          name: layout.name,
                        })}
                        onClick={saveActive}
                      >
                        <Save className='size-4' aria-hidden='true' />
                        {t('shell:controlCenter.layouts.unsaved')}
                      </Button>
                    ) : null}
                    <IconButton
                      variant='ghost'
                      className='min-h-11 min-w-11'
                      label={t('shell:controlCenter.layouts.renameAction', {
                        name: layout.name,
                      })}
                      onClick={() => beginRename(layout)}
                    >
                      <Pencil className='size-4' aria-hidden='true' />
                    </IconButton>
                    <IconButton
                      variant='ghost'
                      className='min-h-11 min-w-11'
                      label={t('shell:controlCenter.layouts.deleteAction', {
                        name: layout.name,
                      })}
                      onClick={() => setConfirmation({ kind: 'delete', layout })}
                    >
                      <Trash2 className='size-4' aria-hidden='true' />
                    </IconButton>
                  </div>
                )}
              </li>
            );
          })}
        </ul>
      )}

      <Dialog open={confirmation !== null} onOpenChange={(open) => !open && setConfirmation(null)}>
        <DialogContent className='w-[min(34rem,92vw)]'>
          <DialogHeader>
            <DialogTitle>
              {confirmation?.kind === 'delete'
                ? t('shell:controlCenter.layouts.deleteTitle')
                : t('shell:controlCenter.layouts.replaceTitle')}
            </DialogTitle>
            <DialogDescription>
              {confirmation?.kind === 'delete'
                ? t('shell:controlCenter.layouts.deleteDescription', {
                    name: confirmation.layout.name,
                  })
                : t('shell:controlCenter.layouts.replaceDescription')}
            </DialogDescription>
          </DialogHeader>
          <DialogBody />
          <DialogFooter>
            <Button variant='ghost' onClick={() => setConfirmation(null)}>
              {t('shell:controlCenter.layouts.cancel')}
            </Button>
            <Button variant='primary' onClick={confirmAction}>
              {confirmation?.kind === 'delete'
                ? t('shell:controlCenter.layouts.deleteConfirm')
                : t('shell:controlCenter.layouts.replaceConfirm')}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
