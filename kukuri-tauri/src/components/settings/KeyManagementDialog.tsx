import { useEffect, useMemo, useState } from 'react';
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

const formatTimestamp = (value: number | null) => {
  if (!value) {
    return '未実施';
  }
  return new Date(value).toLocaleString();
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
      toast.error('ログインしているアカウントが必要です');
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
      toast.success('秘密鍵を取得しました。安全なオフライン環境に保管してください');
    } catch (error) {
      errorHandler.log('Failed to export private key', error, {
        context: 'KeyManagementDialog.handleExport',
        showToast: true,
        toastTitle: '秘密鍵の取得に失敗しました',
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
        toast.warning('繝ｭ繧ｰ繧､繝ｳ保存済みの鍵を表示しました（エクスポートに失敗）');
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
      toast.error('先に秘密鍵を取得してください');
      return;
    }
    setIsSaving(true);
    try {
      const targetPath = await showSaveDialog({
        filters: NSEC_FILE_FILTERS,
        defaultPath: buildDefaultFileName(currentUser?.npub),
        title: '秘密鍵バックアップを保存',
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
      toast.success('バックアップファイルを保存しました');
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
        toastTitle: 'ファイルの保存に失敗しました',
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
      toast.error('先に秘密鍵を取得してください');
      return;
    }
    if (!navigator?.clipboard?.writeText) {
      toast.error('クリップボードへコピーできませんでした');
      return;
    }
    setIsCopying(true);
    try {
      await navigator.clipboard.writeText(exportedKey);
      toast.success('クリップボードへコピーしました（30秒以内に削除してください）');
      recordAction({
        action: 'export',
        status: 'success',
        metadata: { stage: 'copy' },
      });
    } catch (error) {
      errorHandler.log('Failed to copy private key', error, {
        context: 'KeyManagementDialog.handleCopy',
        showToast: true,
        toastTitle: 'コピーに失敗しました',
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
        toast.error('無効なファイルです（nsec1 で始まる秘密鍵を選択してください）');
        recordAction({
          action: 'import',
          status: 'error',
          metadata: { stage: 'read-file', reason: 'invalid_format' },
        });
        return;
      }
      setImportedNsec(trimmed);
      setImportSource(filePath);
      toast.success('秘密鍵ファイルを読み込みました');
    } catch (error) {
      errorHandler.log('Failed to read private key file', error, {
        context: 'KeyManagementDialog.handleSelectFile',
        showToast: true,
        toastTitle: 'ファイルの読み込みに失敗しました',
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
      toast.error('秘密鍵を入力するかファイルを読み込んでください');
      recordAction({
        action: 'import',
        status: 'cancelled',
        metadata: { stage: 'validation' },
      });
      return;
    }
    if (!trimmed.startsWith('nsec1')) {
      toast.error('秘密鍵の形式が正しくありません');
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
      toast.success('秘密鍵をインポートし、セキュアストレージへ保存しました');
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
        toastTitle: 'インポートに失敗しました',
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
          <DialogTitle>鍵管理</DialogTitle>
          <DialogDescription>
            バックアップと復元のフローを実行し、端末紛失時でもアカウントを取り戻せるようにします。
          </DialogDescription>
        </DialogHeader>

        <Tabs
          value={activeTab}
          onValueChange={(value) => setActiveTab(value as 'export' | 'import')}
        >
          <TabsList className="grid grid-cols-2">
            <TabsTrigger value="export" data-testid="key-tab-export">
              エクスポート
            </TabsTrigger>
            <TabsTrigger value="import" data-testid="key-tab-import">
              インポート
            </TabsTrigger>
          </TabsList>

          <TabsContent value="export" className="space-y-4 pt-4">
            <Alert variant="destructive">
              <AlertTitle>誰にも共有しないでください</AlertTitle>
              <AlertDescription>
                秘密鍵はアカウント乗っ取りに直結します。オフライン媒体（USB、紙など）にのみ保存し、オンラインストレージやチャットに貼り付けないでください。
              </AlertDescription>
            </Alert>

            <div className="space-y-3">
              <Button onClick={handleExport} disabled={isExporting} data-testid="key-export-button">
                {isExporting ? '取得中...' : '秘密鍵を取得'}
              </Button>

              {exportedKey && (
                <div className="space-y-3">
                  <Label htmlFor="exported-nsec">秘密鍵</Label>
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
                      {isKeyVisible ? '非表示' : '表示'}
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
                      {isCopying ? 'コピー中...' : 'クリップボードにコピー'}
                    </Button>
                    <Button
                      type="button"
                      variant="outline"
                      onClick={handleSaveToFile}
                      disabled={isSaving}
                      data-testid="key-save-button"
                    >
                      {isSaving ? '保存中...' : 'ファイルに保存'}
                    </Button>
                  </div>
                </div>
              )}

              <div className="rounded-md bg-muted px-3 py-2 text-xs text-muted-foreground">
                最終エクスポート: {formatTimestamp(lastExportedAt)}
              </div>
            </div>
          </TabsContent>

          <TabsContent value="import" className="space-y-4 pt-4">
            <Alert>
              <AlertTitle>バックアップから復元する</AlertTitle>
              <AlertDescription>
                保存済みの `.nsec`
                ファイルを読み込むか、秘密鍵を手動で入力してセキュアストレージに登録します。復旧後は不要なバックアップを破棄してください。
              </AlertDescription>
            </Alert>

            <div className="space-y-3">
              <div className="flex flex-wrap gap-2">
                <Button
                  variant="outline"
                  onClick={handleSelectFile}
                  data-testid="key-import-select-file"
                >
                  鍵ファイルを選択
                </Button>
                {importSource && (
                  <span className="text-sm text-muted-foreground">
                    {sanitizePath(importSource)}
                  </span>
                )}
              </div>

              <div className="space-y-2">
                <Label htmlFor="imported-nsec">秘密鍵を貼り付け</Label>
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
                {isImporting ? 'インポート中...' : 'セキュアストレージに追加'}
              </Button>

              <div className="rounded-md bg-muted px-3 py-2 text-xs text-muted-foreground">
                最終インポート: {formatTimestamp(lastImportedAt)}
              </div>
            </div>
          </TabsContent>
        </Tabs>
      </DialogContent>
    </Dialog>
  );
}
