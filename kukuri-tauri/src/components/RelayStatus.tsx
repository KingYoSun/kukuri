import { useEffect } from 'react';
import { useAuthStore } from '@/stores/authStore';
import { Card, CardHeader, CardTitle, CardContent } from '@/components/ui/card';

export function RelayStatus() {
  const { relayStatus, updateRelayStatus } = useAuthStore();

  useEffect(() => {
    // 初回読み込み時とその後30秒ごとにリレー状態を更新
    updateRelayStatus();
    const interval = setInterval(updateRelayStatus, 30000);

    return () => clearInterval(interval);
  }, [updateRelayStatus]);

  if (relayStatus.length === 0) {
    return null;
  }

  return (
    <Card className="mb-4">
      <CardHeader>
        <CardTitle className="text-sm">リレー接続状態</CardTitle>
      </CardHeader>
      <CardContent>
        <div className="space-y-2">
          {relayStatus.map((relay) => (
            <div key={relay.url} className="flex items-center justify-between text-sm">
              <span className="truncate max-w-[200px]" title={relay.url}>
                {relay.url}
              </span>
              <span
                className={`px-2 py-1 rounded text-xs ${
                  relay.status === 'connected'
                    ? 'bg-green-100 text-green-800'
                    : relay.status === 'connecting'
                      ? 'bg-yellow-100 text-yellow-800'
                      : relay.status === 'disconnected'
                        ? 'bg-gray-100 text-gray-800'
                        : 'bg-red-100 text-red-800'
                }`}
              >
                {relay.status === 'connected'
                  ? '接続済み'
                  : relay.status === 'connecting'
                    ? '接続中'
                    : relay.status === 'disconnected'
                      ? '切断'
                      : 'エラー'}
              </span>
            </div>
          ))}
        </div>
      </CardContent>
    </Card>
  );
}
