import { useEffect, useState } from 'react';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Label } from '@/components/ui/label';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { Separator } from '@/components/ui/separator';
import { errorHandler } from '@/lib/errorHandler';
import { p2pApi } from '@/lib/api/p2p';

type Mode = 'default' | 'custom';

export function BootstrapConfigPanel() {
  const [mode, setMode] = useState<Mode>('default');
  const [nodes, setNodes] = useState<string[]>([]);
  const [newNode, setNewNode] = useState('');
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    (async () => {
      try {
        const raw = await p2pApi.getBootstrapConfig();
        const data = JSON.parse(raw) as { mode: Mode; nodes: string[] };
        setMode(data.mode);
        setNodes(data.nodes || []);
      } catch (e) {
        errorHandler.log('Failed to load bootstrap config', e);
      }
    })();
  }, []);

  const addNode = () => {
    const v = newNode.trim();
    if (!v) return;
    if (!v.includes('@')) {
      errorHandler.log('node_id@host:port の形式で入力してください', undefined, {
        showToast: true,
        toastTitle: '形式エラー',
      });
      return;
    }
    if (nodes.includes(v)) return;
    setNodes((prev) => [...prev, v]);
    setNewNode('');
  };

  const removeNode = (entry: string) => {
    setNodes((prev) => prev.filter((n) => n !== entry));
  };

  const save = async () => {
    try {
      setSaving(true);
      if (mode === 'custom') {
        await p2pApi.setBootstrapNodes(nodes);
      } else {
        await p2pApi.clearBootstrapNodes();
      }
      errorHandler.log(
        mode === 'custom' ? 'カスタムノードを保存しました' : 'デフォルト(n0)に戻しました',
        undefined,
        {
          showToast: true,
          toastTitle: '保存しました',
        },
      );
    } catch (e) {
      errorHandler.log('Failed to save bootstrap config', e, {
        showToast: true,
        toastTitle: '保存に失敗しました',
      });
    } finally {
      setSaving(false);
    }
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle>ブートストラップ設定</CardTitle>
        <CardDescription>
          既定は n0
          提供ノードを利用します。必要に応じてカスタムのブートストラップノード（node_id@host:port）を指定できます。
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="space-y-2">
          <Label>モード</Label>
          <div className="flex items-center gap-3">
            <label className="flex items-center gap-2 text-sm">
              <input
                type="radio"
                name="bootstrap-mode"
                checked={mode === 'default'}
                onChange={() => setMode('default')}
              />
              デフォルト（n0）
            </label>
            <label className="flex items-center gap-2 text-sm">
              <input
                type="radio"
                name="bootstrap-mode"
                checked={mode === 'custom'}
                onChange={() => setMode('custom')}
              />
              カスタム指定
            </label>
          </div>
        </div>

        {mode === 'custom' && (
          <>
            <Separator />
            <div className="space-y-2">
              <Label>ノード（node_id@host:port）</Label>
              <div className="flex gap-2">
                <Input
                  placeholder="npub1...@example.com:11223"
                  value={newNode}
                  onChange={(e) => setNewNode(e.target.value)}
                  className="font-mono"
                />
                <Button onClick={addNode} disabled={!newNode.trim()}>
                  追加
                </Button>
              </div>
              <div className="space-y-2">
                {nodes.length === 0 ? (
                  <p className="text-sm text-muted-foreground">ノードは未指定です</p>
                ) : (
                  nodes.map((n) => (
                    <div
                      key={n}
                      className="flex items-center justify-between rounded-md border px-3 py-2"
                    >
                      <span className="font-mono text-sm truncate">{n}</span>
                      <Button size="sm" variant="ghost" onClick={() => removeNode(n)}>
                        削除
                      </Button>
                    </div>
                  ))
                )}
              </div>
            </div>
          </>
        )}

        <div className="pt-2">
          <Button onClick={save} disabled={saving}>
            {saving ? '保存中...' : '保存'}
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}
