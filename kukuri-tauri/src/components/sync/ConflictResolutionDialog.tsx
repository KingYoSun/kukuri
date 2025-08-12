import { useState } from 'react';
import { SyncConflict } from '@/lib/sync/syncEngine';
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

interface ConflictResolutionDialogProps {
  conflicts: SyncConflict[];
  isOpen: boolean;
  onClose: () => void;
  onResolve: (conflict: SyncConflict, resolution: 'local' | 'remote' | 'merge') => Promise<void>;
}

export function ConflictResolutionDialog({
  conflicts,
  isOpen,
  onClose,
  onResolve,
}: ConflictResolutionDialogProps) {
  const [currentIndex, setCurrentIndex] = useState(0);
  const [selectedResolution, setSelectedResolution] = useState<'local' | 'remote' | 'merge'>('local');
  const [isResolving, setIsResolving] = useState(false);

  const currentConflict = conflicts[currentIndex];

  const handleResolve = async () => {
    if (!currentConflict) return;

    setIsResolving(true);
    try {
      await onResolve(currentConflict, selectedResolution);
      
      // 次の競合に移動、または終了
      if (currentIndex < conflicts.length - 1) {
        setCurrentIndex(currentIndex + 1);
        setSelectedResolution('local');
      } else {
        onClose();
      }
    } catch (error) {
      errorHandler.log('競合解決エラー', error, {
        context: 'ConflictResolutionDialog.handleResolve',
        showToast: true,
        toastTitle: '競合の解決に失敗しました'
      });
    } finally {
      setIsResolving(false);
    }
  };

  const handleSkip = () => {
    if (currentIndex < conflicts.length - 1) {
      setCurrentIndex(currentIndex + 1);
      setSelectedResolution('local');
    } else {
      onClose();
    }
  };

  if (!currentConflict) return null;

  return (
    <Dialog open={isOpen} onOpenChange={onClose}>
      <DialogContent className="max-w-3xl max-h-[80vh]">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <AlertCircle className="h-5 w-5 text-yellow-500" />
            同期の競合を解決
          </DialogTitle>
          <DialogDescription>
            競合 {currentIndex + 1} / {conflicts.length}
          </DialogDescription>
        </DialogHeader>

        <ScrollArea className="max-h-[50vh] pr-4">
          <div className="space-y-4">
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              {/* ローカルの変更 */}
              <Card>
                <CardHeader>
                  <CardTitle className="flex items-center gap-2 text-sm">
                    <Monitor className="h-4 w-4" />
                    ローカルの変更
                  </CardTitle>
                  <CardDescription className="text-xs">
                    あなたのデバイスでの変更
                  </CardDescription>
                </CardHeader>
                <CardContent>
                  <pre className="text-xs bg-muted p-2 rounded overflow-x-auto">
                    {JSON.stringify(currentConflict.localAction, null, 2)}
                  </pre>
                </CardContent>
              </Card>

              {/* リモートの変更 */}
              <Card>
                <CardHeader>
                  <CardTitle className="flex items-center gap-2 text-sm">
                    <Server className="h-4 w-4" />
                    リモートの変更
                  </CardTitle>
                  <CardDescription className="text-xs">
                    他のデバイスまたはサーバーでの変更
                  </CardDescription>
                </CardHeader>
                <CardContent>
                  <pre className="text-xs bg-muted p-2 rounded overflow-x-auto">
                    {JSON.stringify(currentConflict.remoteAction || {}, null, 2)}
                  </pre>
                </CardContent>
              </Card>
            </div>

            {/* マージプレビュー */}
            {currentConflict.mergedData && (
              <Card>
                <CardHeader>
                  <CardTitle className="flex items-center gap-2 text-sm">
                    <GitBranch className="h-4 w-4" />
                    マージ結果のプレビュー
                  </CardTitle>
                  <CardDescription className="text-xs">
                    両方の変更を組み合わせた結果
                  </CardDescription>
                </CardHeader>
                <CardContent>
                  <pre className="text-xs bg-muted p-2 rounded overflow-x-auto">
                    {JSON.stringify(currentConflict.mergedData, null, 2)}
                  </pre>
                </CardContent>
              </Card>
            )}

            {/* 解決方法の選択 */}
            <Card>
              <CardHeader>
                <CardTitle className="text-sm">解決方法を選択</CardTitle>
              </CardHeader>
              <CardContent>
                <RadioGroup
                  value={selectedResolution}
                  onValueChange={(value) => setSelectedResolution(value as 'local' | 'remote' | 'merge')}
                >
                  <div className="flex items-center space-x-2">
                    <RadioGroupItem value="local" id="local" />
                    <Label htmlFor="local" className="cursor-pointer">
                      ローカルの変更を優先する
                    </Label>
                  </div>
                  <div className="flex items-center space-x-2">
                    <RadioGroupItem value="remote" id="remote" />
                    <Label htmlFor="remote" className="cursor-pointer">
                      リモートの変更を優先する
                    </Label>
                  </div>
                  {currentConflict.mergedData && (
                    <div className="flex items-center space-x-2">
                      <RadioGroupItem value="merge" id="merge" />
                      <Label htmlFor="merge" className="cursor-pointer">
                        両方の変更をマージする
                      </Label>
                    </div>
                  )}
                </RadioGroup>
              </CardContent>
            </Card>
          </div>
        </ScrollArea>

        <DialogFooter>
          <Button 
            variant="outline" 
            onClick={handleSkip}
            disabled={isResolving}
          >
            スキップ
          </Button>
          <Button 
            onClick={handleResolve}
            disabled={isResolving}
          >
            {isResolving ? '適用中...' : '適用'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}