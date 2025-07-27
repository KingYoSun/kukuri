import { useP2P } from '@/hooks/useP2P'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { Separator } from '@/components/ui/separator'
import { ScrollArea } from '@/components/ui/scroll-area'
import { 
  WifiIcon, 
  WifiOffIcon, 
  UsersIcon, 
  ServerIcon,
  NetworkIcon,
  AlertCircleIcon,
  CheckCircle2Icon,
  CircleIcon,
} from 'lucide-react'

export function P2PStatus() {
  const {
    initialized,
    nodeId,
    nodeAddr,
    activeTopics,
    peers,
    connectionStatus,
    error,
    clearError,
  } = useP2P()

  // 接続状態のアイコンとカラーを取得
  const getConnectionIcon = () => {
    switch (connectionStatus) {
      case 'connected':
        return <WifiIcon className="h-4 w-4 text-green-500" />
      case 'connecting':
        return <CircleIcon className="h-4 w-4 text-yellow-500 animate-pulse" />
      case 'error':
        return <AlertCircleIcon className="h-4 w-4 text-red-500" />
      default:
        return <WifiOffIcon className="h-4 w-4 text-gray-500" />
    }
  }

  const getConnectionBadge = () => {
    switch (connectionStatus) {
      case 'connected':
        return <Badge variant="default" className="bg-green-500">接続中</Badge>
      case 'connecting':
        return <Badge variant="secondary">接続中...</Badge>
      case 'error':
        return <Badge variant="destructive">エラー</Badge>
      default:
        return <Badge variant="outline">未接続</Badge>
    }
  }

  // 接続中のピア数を計算
  const connectedPeerCount = peers.filter(p => p.connection_status === 'connected').length

  return (
    <Card className="w-full">
      <CardHeader className="pb-3">
        <div className="flex items-center justify-between">
          <CardTitle className="text-sm font-medium">P2P ネットワーク</CardTitle>
          {getConnectionIcon()}
        </div>
        <CardDescription className="text-xs">
          分散型ネットワーク接続状態
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        {/* 接続状態 */}
        <div className="flex items-center justify-between">
          <span className="text-sm text-muted-foreground">状態</span>
          {getConnectionBadge()}
        </div>

        {/* エラー表示 */}
        {error && (
          <div className="bg-red-50 dark:bg-red-950 rounded-md p-3">
            <div className="flex items-start space-x-2">
              <AlertCircleIcon className="h-4 w-4 text-red-500 mt-0.5" />
              <div className="flex-1">
                <p className="text-xs text-red-600 dark:text-red-400">{error}</p>
                <Button
                  variant="ghost"
                  size="sm"
                  className="mt-1 h-6 text-xs"
                  onClick={clearError}
                >
                  閉じる
                </Button>
              </div>
            </div>
          </div>
        )}

        {initialized && connectionStatus === 'connected' && (
          <>
            <Separator />

            {/* ノード情報 */}
            <div className="space-y-2">
              <div className="flex items-center space-x-2 text-xs">
                <ServerIcon className="h-3 w-3 text-muted-foreground" />
                <span className="text-muted-foreground">ノードID</span>
              </div>
              <p className="text-xs font-mono break-all bg-muted/50 rounded px-2 py-1">
                {nodeId?.slice(0, 16)}...
              </p>
            </div>

            {/* ピア情報 */}
            <div className="flex items-center justify-between">
              <div className="flex items-center space-x-2 text-sm">
                <UsersIcon className="h-3 w-3 text-muted-foreground" />
                <span className="text-muted-foreground">接続ピア</span>
              </div>
              <span className="text-sm font-medium">{connectedPeerCount}</span>
            </div>

            {/* アクティブトピック */}
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <div className="flex items-center space-x-2 text-sm">
                  <NetworkIcon className="h-3 w-3 text-muted-foreground" />
                  <span className="text-muted-foreground">参加中のトピック</span>
                </div>
                <span className="text-sm font-medium">{activeTopics.length}</span>
              </div>

              {activeTopics.length > 0 && (
                <ScrollArea className="h-24 w-full rounded-md border">
                  <div className="p-2 space-y-1">
                    {activeTopics.map((topic) => (
                      <div
                        key={topic.topic_id}
                        className="flex items-center justify-between text-xs py-1"
                      >
                        <span className="truncate flex-1 font-mono">
                          {topic.topic_id.slice(0, 8)}...
                        </span>
                        <div className="flex items-center space-x-2 ml-2">
                          <Badge variant="secondary" className="h-5 text-xs">
                            <UsersIcon className="h-2.5 w-2.5 mr-1" />
                            {topic.peer_count}
                          </Badge>
                          <Badge variant="outline" className="h-5 text-xs">
                            {topic.message_count} msgs
                          </Badge>
                        </div>
                      </div>
                    ))}
                  </div>
                </ScrollArea>
              )}
            </div>

            {/* ネットワークアドレス */}
            {nodeAddr && (
              <>
                <Separator />
                <div className="space-y-2">
                  <p className="text-xs text-muted-foreground">ネットワークアドレス</p>
                  <code className="text-xs font-mono break-all bg-muted/50 rounded px-2 py-1 block">
                    {nodeAddr}
                  </code>
                </div>
              </>
            )}
          </>
        )}

        {!initialized && connectionStatus === 'disconnected' && (
          <div className="text-center py-4">
            <WifiOffIcon className="h-8 w-8 text-muted-foreground mx-auto mb-2" />
            <p className="text-sm text-muted-foreground">
              P2Pネットワークに接続していません
            </p>
          </div>
        )}

        {connectionStatus === 'connecting' && (
          <div className="text-center py-4">
            <CircleIcon className="h-8 w-8 text-yellow-500 animate-pulse mx-auto mb-2" />
            <p className="text-sm text-muted-foreground">
              ネットワークに接続中...
            </p>
          </div>
        )}
      </CardContent>
    </Card>
  )
}