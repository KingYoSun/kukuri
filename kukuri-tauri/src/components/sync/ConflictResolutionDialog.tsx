import { useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { SyncConflict } from '@/lib/sync/syncEngine';
import { errorHandler } from '@/lib/errorHandler';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group';
import { Label } from '@/components/ui/label';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { AlertCircle, GitBranch, Server, Monitor } from 'lucide-react';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { cn } from '@/lib/utils';
import {
  extractDocConflictDetails,
  formatBytesValue,
  truncateMiddle,
} from '@/components/sync/conflictUtils';

interface ConflictResolutionDialogProps {
  conflicts: SyncConflict[];
  isOpen: boolean;
  initialIndex?: number;
  onClose: () => void;
  onResolve: (conflict: SyncConflict, resolution: 'local' | 'remote' | 'merge') => Promise<void>;
}

export function ConflictResolutionDialog({
  conflicts,
  isOpen,
  initialIndex = 0,
  onClose,
  onResolve,
}: ConflictResolutionDialogProps) {
  const { t } = useTranslation();
  const [currentIndex, setCurrentIndex] = useState(initialIndex);
  const [selectedResolution, setSelectedResolution] = useState<'local' | 'remote' | 'merge'>(
    'local',
  );
  const [isResolving, setIsResolving] = useState(false);
  const [activeTab, setActiveTab] = useState<'summary' | 'doc'>('summary');

  useEffect(() => {
    if (!isOpen) {
      setCurrentIndex(initialIndex);
      setSelectedResolution('local');
      setActiveTab('summary');
      setIsResolving(false);
    }
  }, [initialIndex, isOpen]);

  useEffect(() => {
    if (!isOpen) {
      return;
    }
    const clamped = Math.min(Math.max(initialIndex, 0), Math.max(conflicts.length - 1, 0));
    setCurrentIndex(clamped);
  }, [initialIndex, conflicts.length, isOpen]);

  useEffect(() => {
    if (!isOpen || conflicts.length === 0) {
      return;
    }
    if (currentIndex >= conflicts.length) {
      setCurrentIndex(conflicts.length - 1);
    }
  }, [conflicts.length, currentIndex, isOpen]);

  const currentConflict = conflicts[currentIndex];
  const localDocDetails = useMemo(
    () => extractDocConflictDetails(currentConflict?.localAction),
    [currentConflict],
  );
  const remoteDocDetails = useMemo(
    () => extractDocConflictDetails(currentConflict?.remoteAction),
    [currentConflict],
  );
  const docComparisonRows = useMemo(
    () => [
      {
        key: 'docVersion',
        label: 'Doc Version',
        local: localDocDetails?.docVersion?.toString(),
        remote: remoteDocDetails?.docVersion?.toString(),
      },
      {
        key: 'blobHash',
        label: 'Blob Hash',
        local: localDocDetails?.blobHash ? truncateMiddle(localDocDetails.blobHash) : undefined,
        remote: remoteDocDetails?.blobHash ? truncateMiddle(remoteDocDetails.blobHash) : undefined,
      },
      {
        key: 'payloadBytes',
        label: 'Payload Size',
        local: formatBytesValue(localDocDetails?.payloadBytes),
        remote: formatBytesValue(remoteDocDetails?.payloadBytes),
      },
      {
        key: 'format',
        label: 'Format',
        local: localDocDetails?.format,
        remote: remoteDocDetails?.format,
      },
      {
        key: 'shareTicket',
        label: 'Share Ticket',
        local: localDocDetails?.shareTicket
          ? truncateMiddle(localDocDetails.shareTicket)
          : undefined,
        remote: remoteDocDetails?.shareTicket
          ? truncateMiddle(remoteDocDetails.shareTicket)
          : undefined,
      },
    ],
    [localDocDetails, remoteDocDetails],
  );
  const showDocTab = docComparisonRows.some((row) => row.local || row.remote);

  useEffect(() => {
    if (!showDocTab && activeTab === 'doc') {
      setActiveTab('summary');
    }
  }, [showDocTab, activeTab]);

  const handleResolve = async () => {
    if (!currentConflict) return;

    setIsResolving(true);
    try {
      await onResolve(currentConflict, selectedResolution);

      // 次の競合に移動、または終了
      if (currentIndex < conflicts.length - 1) {
        setCurrentIndex((index) => index + 1);
        setSelectedResolution('local');
        setActiveTab('summary');
      } else {
        onClose();
      }
    } catch (error) {
      errorHandler.log(t('sync.conflictResolution.resolveError'), error, {
        context: 'ConflictResolutionDialog.handleResolve',
        showToast: true,
        toastTitle: t('sync.conflictResolution.resolveFailed'),
      });
    } finally {
      setIsResolving(false);
    }
  };

  const handleSkip = () => {
    if (currentIndex < conflicts.length - 1) {
      setCurrentIndex((index) => index + 1);
      setSelectedResolution('local');
      setActiveTab('summary');
    } else {
      onClose();
    }
  };

  if (!currentConflict) return null;

  return (
    <Dialog
      open={isOpen}
      onOpenChange={(open) => {
        if (!open) {
          onClose();
        }
      }}
    >
      <DialogContent className="max-w-3xl max-h-[80vh]">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <AlertCircle className="h-5 w-5 text-yellow-500" />
            {t('sync.conflictResolution.title')}
          </DialogTitle>
          <DialogDescription>
            {t('sync.conflictResolution.conflictCount', { current: currentIndex + 1, total: conflicts.length })}
          </DialogDescription>
        </DialogHeader>

        <ScrollArea className="max-h-[50vh] pr-4">
          <Tabs
            value={activeTab}
            onValueChange={(value) => setActiveTab(value as 'summary' | 'doc')}
            className="space-y-4"
          >
            <TabsList
              className={cn('grid w-full gap-2', showDocTab ? 'grid-cols-2' : 'grid-cols-1')}
            >
              <TabsTrigger value="summary">{t('sync.conflictResolution.summary')}</TabsTrigger>
              {showDocTab && <TabsTrigger value="doc">{t('sync.conflictResolution.docBlob')}</TabsTrigger>}
            </TabsList>
            <TabsContent value="summary" className="space-y-4">
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                <Card>
                  <CardHeader>
                    <CardTitle className="flex items-center gap-2 text-sm">
                      <Monitor className="h-4 w-4" />
                      {t('sync.conflictResolution.localChanges')}
                    </CardTitle>
                    <CardDescription className="text-xs">{t('sync.conflictResolution.localChangesDescription')}</CardDescription>
                  </CardHeader>
                  <CardContent>
                    <div className="text-xs text-muted-foreground mb-2 space-y-1">
                      <p>
                        {t('sync.conflictResolution.createdAt')}:{' '}
                        {new Date(currentConflict.localAction.createdAt).toLocaleString()}
                      </p>
                      <p>
                        {t('sync.conflictResolution.type')}:{' '}
                        <code className="rounded bg-muted px-1 py-0.5 font-mono text-[11px] text-foreground">
                          {currentConflict.localAction.actionType}
                        </code>
                      </p>
                    </div>
                    <pre className="text-xs bg-muted p-2 rounded overflow-x-auto">
                      {JSON.stringify(currentConflict.localAction, null, 2)}
                    </pre>
                  </CardContent>
                </Card>

                <Card>
                  <CardHeader>
                    <CardTitle className="flex items-center gap-2 text-sm">
                      <Server className="h-4 w-4" />
                      {t('sync.conflictResolution.remoteChanges')}
                    </CardTitle>
                    <CardDescription className="text-xs">
                      {t('sync.conflictResolution.remoteChangesDescription')}
                    </CardDescription>
                  </CardHeader>
                  <CardContent>
                    {currentConflict.remoteAction ? (
                      <>
                        <div className="text-xs text-muted-foreground mb-2 space-y-1">
                          <p>
                            {t('sync.conflictResolution.createdAt')}:{' '}
                            {new Date(currentConflict.remoteAction.createdAt).toLocaleString()}
                          </p>
                          <p>
                            {t('sync.conflictResolution.type')}:{' '}
                            <code className="rounded bg-muted px-1 py-0.5 font-mono text-[11px] text-foreground">
                              {currentConflict.remoteAction.actionType}
                            </code>
                          </p>
                        </div>
                        <pre className="text-xs bg-muted p-2 rounded overflow-x-auto">
                          {JSON.stringify(currentConflict.remoteAction, null, 2)}
                        </pre>
                      </>
                    ) : (
                      <p className="text-xs text-muted-foreground">
                        {t('sync.conflictResolution.noRemoteChanges')}
                      </p>
                    )}
                  </CardContent>
                </Card>
              </div>

              {currentConflict.mergedData && (
                <Card>
                  <CardHeader>
                    <CardTitle className="flex items-center gap-2 text-sm">
                      <GitBranch className="h-4 w-4" />
                      {t('sync.conflictResolution.mergePreview')}
                    </CardTitle>
                    <CardDescription className="text-xs">
                      {t('sync.conflictResolution.mergePreviewDescription')}
                    </CardDescription>
                  </CardHeader>
                  <CardContent>
                    <pre className="text-xs bg-muted p-2 rounded overflow-x-auto">
                      {JSON.stringify(currentConflict.mergedData, null, 2)}
                    </pre>
                  </CardContent>
                </Card>
              )}

              <Card>
                <CardHeader>
                  <CardTitle className="text-sm">{t('sync.conflictResolution.selectResolution')}</CardTitle>
                </CardHeader>
                <CardContent>
                  <RadioGroup
                    value={selectedResolution}
                    onValueChange={(value: string) =>
                      setSelectedResolution(value as 'local' | 'remote' | 'merge')
                    }
                    className="space-y-3"
                  >
                    <div className="flex items-center space-x-2">
                      <RadioGroupItem value="local" id="local" />
                      <Label htmlFor="local" className="cursor-pointer">
                        {t('sync.conflictResolution.preferLocal')}
                      </Label>
                    </div>
                    <div className="flex items-center space-x-2">
                      <RadioGroupItem
                        value="remote"
                        id="remote"
                        disabled={!currentConflict.remoteAction}
                      />
                      <Label
                        htmlFor="remote"
                        className={cn(
                          'cursor-pointer',
                          !currentConflict.remoteAction && 'text-muted-foreground',
                        )}
                      >
                        {t('sync.conflictResolution.preferRemote')}
                      </Label>
                    </div>
                    {currentConflict.mergedData && (
                      <div className="flex items-center space-x-2">
                        <RadioGroupItem value="merge" id="merge" />
                        <Label htmlFor="merge" className="cursor-pointer">
                          {t('sync.conflictResolution.mergeBoth')}
                        </Label>
                      </div>
                    )}
                  </RadioGroup>
                </CardContent>
              </Card>
            </TabsContent>

            {showDocTab && (
              <TabsContent value="doc">
                {docComparisonRows.every((row) => !row.local && !row.remote) ? (
                  <p className="text-sm text-muted-foreground">{t('sync.conflictResolution.noDocBlobInfo')}</p>
                ) : (
                  <div className="space-y-3">
                    {docComparisonRows.map((row) => {
                      const differ =
                        row.local !== undefined &&
                        row.remote !== undefined &&
                        row.local !== row.remote;
                      return (
                        <div key={row.key} className="text-sm rounded border p-2">
                          <p className="text-xs uppercase text-muted-foreground mb-1">
                            {row.label}
                          </p>
                          <div className="grid grid-cols-2 gap-3 text-xs">
                            <div>
                              <p className="text-muted-foreground mb-0.5">{t('sync.conflictResolution.local')}</p>
                              <p
                                className={cn('font-medium break-all', differ && 'text-amber-600')}
                              >
                                {row.local ?? '—'}
                              </p>
                            </div>
                            <div>
                              <p className="text-muted-foreground mb-0.5">{t('sync.conflictResolution.remote')}</p>
                              <p
                                className={cn('font-medium break-all', differ && 'text-amber-600')}
                              >
                                {row.remote ?? '—'}
                              </p>
                            </div>
                          </div>
                        </div>
                      );
                    })}
                  </div>
                )}
              </TabsContent>
            )}
          </Tabs>
        </ScrollArea>

        <DialogFooter>
          <Button variant="outline" onClick={handleSkip} disabled={isResolving}>
            {t('sync.conflictResolution.skip')}
          </Button>
          <Button onClick={handleResolve} disabled={isResolving}>
            {isResolving ? t('sync.conflictResolution.applying') : t('sync.conflictResolution.apply')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
