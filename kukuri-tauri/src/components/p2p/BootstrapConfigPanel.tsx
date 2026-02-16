import { useTranslation } from 'react-i18next';
import { useEffect, useState } from 'react';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Label } from '@/components/ui/label';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { Separator } from '@/components/ui/separator';
import { errorHandler } from '@/lib/errorHandler';
import { p2pApi } from '@/lib/api/p2p';

type Mode = 'default' | 'custom';

const sourceKeys = {
  env: 'sourceEnv',
  user: 'sourceUser',
  bundle: 'sourceBundle',
  fallback: 'sourceFallback',
  none: 'sourceNone',
} as const;

export function BootstrapConfigPanel() {
  const { t } = useTranslation();
  const [mode, setMode] = useState<Mode>('default');
  const [nodes, setNodes] = useState<string[]>([]);
  const [effectiveNodes, setEffectiveNodes] = useState<string[]>([]);
  const [source, setSource] = useState<'env' | 'user' | 'bundle' | 'fallback' | 'none'>('none');
  const [envLocked, setEnvLocked] = useState(false);
  const [newNode, setNewNode] = useState('');
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    (async () => {
      try {
        const data = await p2pApi.getBootstrapConfig();
        setMode(data.mode as Mode);
        setNodes(data.nodes ?? []);
        setEffectiveNodes(data.effective_nodes ?? []);
        setSource(data.source ?? 'none');
        setEnvLocked(Boolean(data.env_locked));
      } catch (e) {
        errorHandler.log('Failed to load bootstrap config', e);
      }
    })();
  }, []);

  const sourceLabel = t(`bootstrapPanel.${sourceKeys[source]}`);

  const handleSetMode = (value: Mode) => {
    if (envLocked) return;
    setMode(value);
  };

  const addNode = () => {
    if (envLocked) {
      return;
    }
    const v = newNode.trim();
    if (!v) return;
    if (!v.includes('@')) {
      errorHandler.log(t('bootstrapPanel.formatHint'), undefined, {
        showToast: true,
        toastTitle: t('bootstrapPanel.formatError'),
      });
      return;
    }
    if (nodes.includes(v)) return;
    setNodes((prev) => [...prev, v]);
    setNewNode('');
  };

  const removeNode = (entry: string) => {
    if (envLocked) {
      return;
    }
    setNodes((prev) => prev.filter((n) => n !== entry));
  };

  const save = async () => {
    if (envLocked) {
      errorHandler.log(t('bootstrapPanel.envLockedMessage'), undefined, {
        showToast: true,
        toastTitle: t('bootstrapPanel.envLocked'),
      });
      return;
    }
    try {
      setSaving(true);
      if (mode === 'custom') {
        await p2pApi.setBootstrapNodes(nodes);
      } else {
        await p2pApi.clearBootstrapNodes();
      }
      const refreshed = await p2pApi.getBootstrapConfig();
      setMode(refreshed.mode as Mode);
      setNodes(refreshed.nodes ?? []);
      setEnvLocked(Boolean(refreshed.env_locked));
      setEffectiveNodes(refreshed.effective_nodes ?? []);
      setSource(refreshed.source ?? 'none');
      errorHandler.log(
        mode === 'custom' ? t('bootstrapPanel.saved') : t('bootstrapPanel.saved'),
        undefined,
        {
          showToast: true,
          toastTitle: t('bootstrapPanel.saved'),
        },
      );
    } catch (e) {
      errorHandler.log('Failed to save bootstrap config', e, {
        showToast: true,
        toastTitle: t('bootstrapPanel.saveFailed'),
      });
    } finally {
      setSaving(false);
    }
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t('bootstrapPanel.title')}</CardTitle>
        <CardDescription>{t('bootstrapPanel.description')}</CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="space-y-2">
          <Label>{t('bootstrapPanel.appliedNodes')}</Label>
          {effectiveNodes.length === 0 ? (
            <p className="text-sm text-muted-foreground">{t('bootstrapPanel.usingN0')}</p>
          ) : (
            <div className="space-y-2">
              {effectiveNodes.map((n) => (
                <div key={n} className="rounded-md border px-3 py-2 font-mono text-sm truncate">
                  {n}
                </div>
              ))}
            </div>
          )}
          <p className="text-xs text-muted-foreground">
            {t('bootstrapPanel.source')}: {sourceLabel}
          </p>
          {envLocked && (
            <p className="text-xs text-muted-foreground">{t('bootstrapPanel.envLockedHint')}</p>
          )}
        </div>

        <div className="space-y-2">
          <Label>{t('bootstrapPanel.mode')}</Label>
          <div className="flex items-center gap-3">
            <label className="flex items-center gap-2 text-sm">
              <input
                type="radio"
                name="bootstrap-mode"
                checked={mode === 'default'}
                onChange={() => handleSetMode('default')}
                disabled={envLocked}
              />
              {t('bootstrapPanel.modeDefault')}
            </label>
            <label className="flex items-center gap-2 text-sm">
              <input
                type="radio"
                name="bootstrap-mode"
                checked={mode === 'custom'}
                onChange={() => handleSetMode('custom')}
                disabled={envLocked}
              />
              {t('bootstrapPanel.modeCustom')}
            </label>
          </div>
        </div>

        {mode === 'custom' && (
          <>
            <Separator />
            <div className="space-y-2">
              <Label>{t('bootstrapPanel.nodesLabel')}</Label>
              <div className="flex gap-2">
                <Input
                  placeholder="npub1...@example.com:11223"
                  value={newNode}
                  onChange={(e) => setNewNode(e.target.value)}
                  className="font-mono"
                  disabled={envLocked}
                />
                <Button onClick={addNode} disabled={envLocked || !newNode.trim()}>
                  {t('bootstrapConfig.add')}
                </Button>
              </div>
              <div className="space-y-2">
                {nodes.length === 0 ? (
                  <p className="text-sm text-muted-foreground">{t('bootstrapConfig.noNodes')}</p>
                ) : (
                  nodes.map((n) => (
                    <div
                      key={n}
                      className="flex items-center justify-between rounded-md border px-3 py-2"
                    >
                      <span className="font-mono text-sm truncate">{n}</span>
                      <Button
                        size="sm"
                        variant="ghost"
                        onClick={() => removeNode(n)}
                        disabled={envLocked}
                      >
                        {t('common.delete')}
                      </Button>
                    </div>
                  ))
                )}
              </div>
            </div>
          </>
        )}

        <div className="pt-2">
          <Button onClick={save} disabled={saving || envLocked}>
            {envLocked
              ? t('bootstrapPanel.envLocked')
              : saving
                ? t('bootstrapPanel.saving')
                : t('bootstrapPanel.save')}
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}
