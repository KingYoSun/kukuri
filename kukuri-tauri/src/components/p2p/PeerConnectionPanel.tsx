import { useTranslation } from 'react-i18next';
import { useState, useEffect } from 'react';
import { useP2PStore } from '@/stores/p2pStore';
import { p2pApi } from '@/lib/api/p2p';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { useToast } from '@/hooks/use-toast';
import { Copy, Loader2, WifiIcon, WifiOff } from 'lucide-react';
import { Separator } from '@/components/ui/separator';
import { ScrollArea } from '@/components/ui/scroll-area';
import { errorHandler } from '@/lib/errorHandler';

interface PeerConnection {
  id: string;
  address: string;
  timestamp: number;
  status: 'connected' | 'failed';
}

export function PeerConnectionPanel() {
  const { t } = useTranslation();
  const { nodeAddr, connectionStatus, initialize } = useP2PStore();
  const { toast } = useToast();
  const [peerAddress, setPeerAddress] = useState('');
  const [isConnecting, setIsConnecting] = useState(false);
  const [connectionHistory, setConnectionHistory] = useState<PeerConnection[]>([]);

  // 初期化
  useEffect(() => {
    if (!nodeAddr && connectionStatus === 'disconnected') {
      initialize();
    }
  }, [nodeAddr, connectionStatus, initialize]);

  // ローカルストレージから接続履歴を読み込み
  useEffect(() => {
    const storedHistory = localStorage.getItem('p2p-connection-history');
    if (storedHistory) {
      try {
        setConnectionHistory(JSON.parse(storedHistory));
      } catch (error) {
        errorHandler.log('Failed to load connection history', error);
      }
    }
  }, []);

  // 接続履歴を保存
  const saveConnectionHistory = (history: PeerConnection[]) => {
    try {
      localStorage.setItem('p2p-connection-history', JSON.stringify(history));
    } catch (error) {
      errorHandler.log('Failed to save connection history', error);
    }
  };

  // クリップボードにコピー
  const copyToClipboard = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
      toast({
        title: t('p2pPanel.copied'),
        description: t('p2pPanel.copyDesc'),
      });
    } catch (error) {
      errorHandler.log('Failed to copy to clipboard', error, {
        showToast: true,
        toastTitle: t('p2pPanel.copyFailed'),
      });
    }
  };

  // ピアアドレスのバリデーション
  const validatePeerAddress = (address: string): boolean => {
    // 基本的なフォーマット検証
    // /ip4/xxx.xxx.xxx.xxx/tcp/xxxx/p2p/QmXXX... または
    // /ip6/xxxx:xxxx:xxxx:xxxx:xxxx:xxxx:xxxx:xxxx/tcp/xxxx/p2p/QmXXX...
    const ipv4Pattern = /^\/ip4\/\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\/tcp\/\d+\/p2p\/[a-zA-Z0-9]+$/;
    const ipv6Pattern = /^\/ip6\/[a-fA-F0-9:]+\/tcp\/\d+\/p2p\/[a-zA-Z0-9]+$/;

    return ipv4Pattern.test(address) || ipv6Pattern.test(address);
  };

  // ピアに接続
  const handleConnect = async () => {
    const trimmedAddress = peerAddress.trim();

    if (!trimmedAddress) {
      toast({
        title: t('common.error'),
        description: t('p2pPanel.errorEnterAddress'),
        variant: 'destructive',
      });
      return;
    }

    if (!validatePeerAddress(trimmedAddress)) {
      toast({
        title: t('common.error'),
        description: t('p2pPanel.errorInvalidAddress'),
        variant: 'destructive',
      });
      return;
    }

    setIsConnecting(true);
    const newConnection: PeerConnection = {
      id: crypto.randomUUID(),
      address: trimmedAddress,
      timestamp: Date.now(),
      status: 'connected',
    };

    try {
      await p2pApi.connectToPeer(trimmedAddress);

      toast({
        title: t('p2pPanel.connectSuccess'),
        description: t('p2pPanel.connectSuccessDesc'),
      });

      // 接続履歴に追加（最大10件）
      const updatedHistory = [newConnection, ...connectionHistory]
        .filter((conn, index, self) => index === self.findIndex((c) => c.address === conn.address))
        .slice(0, 10);

      setConnectionHistory(updatedHistory);
      saveConnectionHistory(updatedHistory);
      setPeerAddress('');
    } catch (error) {
      newConnection.status = 'failed';

      const updatedHistory = [newConnection, ...connectionHistory]
        .filter((conn, index, self) => index === self.findIndex((c) => c.address === conn.address))
        .slice(0, 10);

      setConnectionHistory(updatedHistory);
      saveConnectionHistory(updatedHistory);

      errorHandler.log('Failed to connect to peer', error, {
        showToast: true,
        toastTitle: t('p2pPanel.connectFailed'),
      });
    } finally {
      setIsConnecting(false);
    }
  };

  // 接続履歴から再接続
  const handleReconnect = async (address: string) => {
    setPeerAddress(address);
    await handleConnect();
  };

  // 接続履歴をクリア
  const clearHistory = () => {
    setConnectionHistory([]);
    localStorage.removeItem('p2p-connection-history');
    toast({
      title: t('p2pPanel.historyCleared'),
      description: t('p2pPanel.historyClearedDesc'),
    });
  };

  return (
    <Card className="w-full">
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          {connectionStatus === 'connected' ? (
            <WifiIcon className="h-5 w-5 text-green-500" />
          ) : (
            <WifiOff className="h-5 w-5 text-red-500" />
          )}
          {t('p2pPanel.title')}
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-6">
        <div className="space-y-2">
          <Label>{t('p2pPanel.yourAddress')}</Label>
          {nodeAddr ? (
            <div className="flex gap-2">
              <Input value={nodeAddr} readOnly className="font-mono text-sm" />
              <Button
                size="icon"
                variant="outline"
                onClick={() => copyToClipboard(nodeAddr)}
                title={t('p2pPanel.copy')}
              >
                <Copy className="h-4 w-4" />
              </Button>
            </div>
          ) : (
            <div className="flex items-center gap-2 text-muted-foreground">
              <Loader2 className="h-4 w-4 animate-spin" />
              {t('p2pPanel.fetchingAddress')}
            </div>
          )}
          <p className="text-xs text-muted-foreground">{t('p2pPanel.shareHint')}</p>
        </div>

        <Separator />

        <div className="space-y-2">
          <Label>{t('p2pPanel.connectToPeer')}</Label>
          <div className="flex gap-2">
            <Input
              value={peerAddress}
              onChange={(e) => setPeerAddress(e.target.value)}
              placeholder={t('p2pPanel.peerAddressPlaceholder')}
              disabled={isConnecting}
              className="font-mono text-sm"
              onKeyDown={(e) => {
                if (e.key === 'Enter' && !isConnecting) {
                  handleConnect();
                }
              }}
            />
            <Button onClick={handleConnect} disabled={isConnecting || !peerAddress.trim()}>
              {isConnecting ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  {t('p2pPanel.connecting')}
                </>
              ) : (
                t('p2pPanel.connect')
              )}
            </Button>
          </div>
          <p className="text-xs text-muted-foreground">{t('p2pPanel.peerAddressHint')}</p>
        </div>

        {connectionHistory.length > 0 && (
          <>
            <Separator />
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <Label>{t('p2pPanel.connectionHistory')}</Label>
                <Button variant="ghost" size="sm" onClick={clearHistory} className="text-xs">
                  {t('p2pPanel.clearHistory')}
                </Button>
              </div>
              <ScrollArea className="h-[200px] w-full rounded-md border p-4">
                <div className="space-y-2">
                  {connectionHistory.map((conn) => (
                    <div
                      key={conn.id}
                      className="flex items-center justify-between py-2 px-3 rounded-md hover:bg-accent"
                    >
                      <div className="flex-1 min-w-0">
                        <p className="text-sm font-mono truncate">{conn.address}</p>
                        <p className="text-xs text-muted-foreground">
                          {new Date(conn.timestamp).toLocaleString()}
                          {conn.status === 'failed' && (
                            <span className="ml-2 text-red-500">
                              {t('p2pPanel.connectionFailed')}
                            </span>
                          )}
                        </p>
                      </div>
                      <Button
                        size="sm"
                        variant="ghost"
                        onClick={() => handleReconnect(conn.address)}
                      >
                        {t('p2pPanel.reconnect')}
                      </Button>
                    </div>
                  ))}
                </div>
              </ScrollArea>
            </div>
          </>
        )}
      </CardContent>
    </Card>
  );
}
