import { useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { open as showOpenDialog, save as showSaveDialog } from '@tauri-apps/plugin-dialog';
import { readTextFile, writeTextFile } from '@tauri-apps/plugin-fs';
import { toast } from 'sonner';

import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import { errorHandler } from '@/lib/errorHandler';
import { TauriApi } from '@/lib/api/tauri';
import { formatDateTimeByI18n } from '@/lib/utils/localeFormat';
import { useAuthStore } from '@/stores/authStore';
import { useKeyManagementStore } from '@/stores/keyManagementStore';

interface KeyManagementDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

const NSEC_FILE_FILTERS = [
  {
    name: 'Nostr Secret Key',
    extensions: ['nsec'],
  },
];

const formatTimestamp = (value: number | null, t: (key: string) => string) => {
  if (!value) {
    return t('settings.account.notPerformed');
  }
  return formatDateTimeByI18n(value);
};

const buildDefaultFileName = (npub?: string | null) => {
  if (!npub) {
    return 'kukuri_key_backup.nsec';
  }
  return `kukuri_key_backup_${npub.slice(0, 10)}.nsec`;
};

const sanitizePath = (filePath: string) => {
  const segments = filePath.split(/\\|\//);
  return segments[segments.length - 1] ?? 'backup.nsec';
};

const anonymizeNpub = (npub?: string | null) => {
  if (!npub) {
    return 'unknown';
  }
  return `${npub.slice(0, 8)}…${npub.slice(-4)}`;
};

export function KeyManagementDialog({ open, onOpenChange }: KeyManagementDialogProps) {
  const { t } = useTranslation();
  const currentUser = useAuthStore((state) => state.currentUser);
  const loginWithNsec = useAuthStore((state) => state.loginWithNsec);
  const recordAction = useKeyManagementStore((state) => state.recordAction);
  const lastExportedAt = useKeyManagementStore((state) => state.lastExportedAt);
  const lastImportedAt = useKeyManagementStore((state) => state.lastImportedAt);

  const [activeTab, setActiveTab] = useState<'export' | 'import'>('export');
  const [exportedKey, setExportedKey] = useState<string | null>(null);
  const [isExporting, setIsExporting] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [isCopying, setIsCopying] = useState(false);
  const [isKeyVisible, setIsKeyVisible] = useState(false);
  const [importedNsec, setImportedNsec] = useState('');
  const [importSource, setImportSource] = useState<string | null>(null);
  const [isImporting, setIsImporting] = useState(false);

  useEffect(() => {
    if (!open) {
      setActiveTab('export');
      setExportedKey(null);
      setImportedNsec('');
      setImportSource(null);
      setIsKeyVisible(false);
    }
  }, [open]);

  const canImport = useMemo(() => importedNsec.trim().startsWith('nsec1'), [importedNsec]);

  const handleExport = async () => {
    if (!currentUser?.npub) {
      toast.error(t('settings.account.loginRequired'));
      return;
    }
    setIsExporting(true);
    try {
      const nsec = await TauriApi.exportPrivateKey(currentUser.npub);
      setExportedKey(nsec);
      setIsKeyVisible(false);
      recordAction({
        action: 'export',
        status: 'success',
        metadata: { stage: 'fetch', npub: anonymizeNpub(currentUser.npub) },
      });
      errorHandler.info('Private key exported successfully', 'KeyManagementDialog.export', {
        npub: currentUser.npub,
      });
      toast.success(t('settings.account.exportSuccess'));
    } catch (error) {
      errorHandler.log('Failed to export private key', error, {
        context: 'KeyManagementDialog.handleExport',
        showToast: true,
        toastTitle: t('settings.account.exportFailed'),
      });
      recordAction({
        action: 'export',
        status: 'error',
        metadata: { stage: 'fetch' },
      });
      const fallbackNsec = useAuthStore.getState().privateKey;
      if (fallbackNsec) {
        setExportedKey(fallbackNsec);
        setIsKeyVisible(false);
        toast.warning(t('settings.account.exportFallback'));
        recordAction({
          action: 'export',
          status: 'success',
          metadata: { stage: 'fallback', source: 'authStore.privateKey' },
        });
        return;
      }
    } finally {
      setIsExporting(false);
    }
  };

  const handleSaveToFile = async () => {
    if (!exportedKey) {
      toast.error(t('settings.account.exportFirst'));
      return;
    }
    setIsSaving(true);
    try {
      const targetPath = await showSaveDialog({
        filters: NSEC_FILE_FILTERS,
        defaultPath: buildDefaultFileName(currentUser?.npub),
        title: t('settings.account.saveBackupTitle'),
      });
      if (!targetPath) {
        recordAction({
          action: 'export',
          status: 'cancelled',
          metadata: { stage: 'save-dialog' },
        });
        return;
      }
      await writeTextFile(targetPath, exportedKey);
      toast.success(t('settings.account.saveSuccess'));
      recordAction({
        action: 'export',
        status: 'success',
        metadata: { stage: 'save-file', destination: sanitizePath(targetPath) },
      });
      errorHandler.info('Private key saved to file', 'KeyManagementDialog.save', {
        destination: sanitizePath(targetPath),
      });
    } catch (error) {
      errorHandler.log('Failed to save private key', error, {
        context: 'KeyManagementDialog.handleSaveToFile',
        showToast: true,
        toastTitle: t('settings.account.saveFailed'),
      });
      recordAction({
        action: 'export',
        status: 'error',
        metadata: { stage: 'save-file' },
      });
    } finally {
      setIsSaving(false);
    }
  };

  const handleCopy = async () => {
    if (!exportedKey) {
      toast.error(t('settings.account.exportFirst'));
      return;
    }
    if (!navigator?.clipboard?.writeText) {
      toast.error(t('settings.account.copyNotSupported'));
      return;
    }
    setIsCopying(true);
    try {
      await navigator.clipboard.writeText(exportedKey);
      toast.success(t('settings.account.copySuccess'));
      recordAction({
        action: 'export',
        status: 'success',
        metadata: { stage: 'copy' },
      });
    } catch (error) {
      errorHandler.log('Failed to copy private key', error, {
        context: 'KeyManagementDialog.handleCopy',
        showToast: true,
        toastTitle: t('settings.account.copyFailed'),
      });
      recordAction({
        action: 'export',
        status: 'error',
        metadata: { stage: 'copy' },
      });
    } finally {
      setIsCopying(false);
    }
  };

  const handleSelectFile = async () => {
    try {
      const selection = await showOpenDialog({
        directory: false,
        multiple: false,
        filters: NSEC_FILE_FILTERS,
      });
      if (!selection) {
        return;
      }
      const filePath = Array.isArray(selection) ? selection[0] : selection;
      const contents = await readTextFile(filePath);
      const trimmed = contents.trim();
      if (!trimmed.startsWith('nsec1')) {
        toast.error(t('settings.account.invalidFile'));
        recordAction({
          action: 'import',
          status: 'error',
          metadata: { stage: 'read-file', reason: 'invalid_format' },
        });
        return;
      }
      setImportedNsec(trimmed);
      setImportSource(filePath);
      toast.success(t('settings.account.fileLoaded'));
    } catch (error) {
      errorHandler.log('Failed to read private key file', error, {
        context: 'KeyManagementDialog.handleSelectFile',
        showToast: true,
        toastTitle: t('settings.account.fileLoadFailed'),
      });
      recordAction({
        action: 'import',
        status: 'error',
        metadata: { stage: 'read-file' },
      });
    }
  };

  const handleImport = async () => {
    const trimmed = importedNsec.trim();
    if (!trimmed) {
      toast.error(t('settings.account.enterOrLoadKey'));
      recordAction({
        action: 'import',
        status: 'cancelled',
        metadata: { stage: 'validation' },
      });
      return;
    }
    if (!trimmed.startsWith('nsec1')) {
      toast.error(t('settings.account.invalidFormat'));
      recordAction({
        action: 'import',
        status: 'error',
        metadata: { stage: 'validation', reason: 'invalid_format' },
      });
      return;
    }
    setIsImporting(true);
    try {
      await loginWithNsec(trimmed, true);
      toast.success(t('settings.account.importSuccess'));
      recordAction({
        action: 'import',
        status: 'success',
        metadata: { source: importSource ? 'file' : 'manual' },
      });
      errorHandler.info('Private key imported successfully', 'KeyManagementDialog.import', {
        source: importSource ? sanitizePath(importSource) : 'manual',
      });
      setImportedNsec('');
      setImportSource(null);
    } catch (error) {
      errorHandler.log('Failed to import private key', error, {
        context: 'KeyManagementDialog.handleImport',
        showToast: true,
        toastTitle: t('settings.account.importFailed'),
      });
      recordAction({
        action: 'import',
        status: 'error',
        metadata: { stage: 'login' },
      });
    } finally {
      setIsImporting(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl" data-testid="key-management-dialog">
        <DialogHeader className="space-y-2">
          <DialogTitle>{t('settings.account.keyManagementTitle')}</DialogTitle>
          <DialogDescription>
            {t('settings.account.keyManagementDescriptionFull')}
          </DialogDescription>
        </DialogHeader>

        <Tabs
          value={activeTab}
          onValueChange={(value) => setActiveTab(value as 'export' | 'import')}
        >
          <TabsList className="grid grid-cols-2">
            <TabsTrigger value="export" data-testid="key-tab-export">
              {t('settings.account.export')}
            </TabsTrigger>
            <TabsTrigger value="import" data-testid="key-tab-import">
              {t('settings.account.import')}
            </TabsTrigger>
          </TabsList>

          <TabsContent value="export" className="space-y-4 pt-4">
            <Alert variant="destructive">
              <AlertTitle>{t('settings.account.doNotShare')}</AlertTitle>
              <AlertDescription>{t('settings.account.doNotShareDescription')}</AlertDescription>
            </Alert>

            <div className="space-y-3">
              <Button onClick={handleExport} disabled={isExporting} data-testid="key-export-button">
                {isExporting ? t('settings.account.exporting') : t('settings.account.exportKey')}
              </Button>

              {exportedKey && (
                <div className="space-y-3">
                  <Label htmlFor="exported-nsec">{t('settings.account.privateKey')}</Label>
                  <div className="flex gap-2">
                    <Input
                      id="exported-nsec"
                      type={isKeyVisible ? 'text' : 'password'}
                      value={exportedKey}
                      readOnly
                      data-testid="key-exported-value"
                    />
                    <Button
                      type="button"
                      variant="outline"
                      onClick={() => setIsKeyVisible((value) => !value)}
                      data-testid="key-toggle-visibility"
                    >
                      {isKeyVisible ? t('settings.account.hide') : t('settings.account.show')}
                    </Button>
                  </div>
                  <div className="flex flex-wrap gap-2">
                    <Button
                      type="button"
                      variant="outline"
                      onClick={handleCopy}
                      disabled={isCopying}
                      data-testid="key-copy-button"
                    >
                      {isCopying
                        ? t('settings.account.copying')
                        : t('settings.account.copyToClipboard')}
                    </Button>
                    <Button
                      type="button"
                      variant="outline"
                      onClick={handleSaveToFile}
                      disabled={isSaving}
                      data-testid="key-save-button"
                    >
                      {isSaving ? t('settings.account.saving') : t('settings.account.saveToFile')}
                    </Button>
                  </div>
                </div>
              )}

              <div className="rounded-md bg-muted px-3 py-2 text-xs text-muted-foreground">
                {t('settings.account.lastExport')}: {formatTimestamp(lastExportedAt, t)}
              </div>
            </div>
          </TabsContent>

          <TabsContent value="import" className="space-y-4 pt-4">
            <Alert>
              <AlertTitle>{t('settings.account.restoreFromBackup')}</AlertTitle>
              <AlertDescription>{t('settings.account.restoreDescription')}</AlertDescription>
            </Alert>

            <div className="space-y-3">
              <div className="flex flex-wrap gap-2">
                <Button
                  variant="outline"
                  onClick={handleSelectFile}
                  data-testid="key-import-select-file"
                >
                  {t('settings.account.selectKeyFile')}
                </Button>
                {importSource && (
                  <span className="text-sm text-muted-foreground">
                    {sanitizePath(importSource)}
                  </span>
                )}
              </div>

              <div className="space-y-2">
                <Label htmlFor="imported-nsec">{t('settings.account.pastePrivateKey')}</Label>
                <Textarea
                  id="imported-nsec"
                  rows={3}
                  value={importedNsec}
                  onChange={(event) => setImportedNsec(event.target.value)}
                  placeholder="nsec1..."
                  data-testid="key-import-input"
                />
              </div>

              <Button
                onClick={handleImport}
                disabled={isImporting || !canImport}
                data-testid="key-import-button"
              >
                {isImporting
                  ? t('settings.account.importing')
                  : t('settings.account.addToSecureStorage')}
              </Button>

              <div className="rounded-md bg-muted px-3 py-2 text-xs text-muted-foreground">
                {t('settings.account.lastImport')}: {formatTimestamp(lastImportedAt, t)}
              </div>
            </div>
          </TabsContent>
        </Tabs>
      </DialogContent>
    </Dialog>
  );
}
