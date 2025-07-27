import { useEffect } from 'react'
import { listen } from '@tauri-apps/api/event'
import { useP2PStore, type P2PMessage, type PeerInfo } from '@/stores/p2pStore'

// P2Pイベントの型定義
interface P2PMessageEvent {
  topic_id: string
  message: {
    id: string
    author: string
    content: string
    timestamp: number
    signature: string
  }
}

interface P2PPeerEvent {
  topic_id: string
  peer_id: string
  event_type: 'joined' | 'left'
}

interface P2PConnectionEvent {
  node_id: string
  node_addr: string
  status: 'connected' | 'disconnected'
}

// P2Pイベントリスナーフック
export function useP2PEventListener() {
  const {
    addMessage,
    updatePeer,
    removePeer,
    refreshStatus,
  } = useP2PStore()

  useEffect(() => {
    const unlisteners: Promise<() => void>[] = []

    // P2Pメッセージ受信
    unlisteners.push(
      listen<P2PMessageEvent>('p2p://message', (event) => {
        const { topic_id, message } = event.payload
        
        const p2pMessage: P2PMessage = {
          ...message,
          topic_id,
        }
        
        addMessage(p2pMessage)
      })
    )

    // ピアイベント（参加/離脱）
    unlisteners.push(
      listen<P2PPeerEvent>('p2p://peer', (event) => {
        const { topic_id, peer_id, event_type } = event.payload
        
        if (event_type === 'joined') {
          // ピア参加時の処理
          const peerInfo: PeerInfo = {
            node_id: peer_id,
            node_addr: '', // 後で実際のアドレスを取得
            topics: [topic_id],
            last_seen: Date.now(),
            connection_status: 'connected',
          }
          updatePeer(peerInfo)
        } else if (event_type === 'left') {
          // ピア離脱時の処理
          removePeer(peer_id)
        }
        
        // 状態を更新
        refreshStatus()
      })
    )

    // 接続状態イベント
    unlisteners.push(
      listen<P2PConnectionEvent>('p2p://connection', (event) => {
        const { node_id, node_addr, status } = event.payload
        
        if (status === 'connected') {
          const peerInfo: PeerInfo = {
            node_id,
            node_addr,
            topics: [],
            last_seen: Date.now(),
            connection_status: 'connected',
          }
          updatePeer(peerInfo)
        } else {
          removePeer(node_id)
        }
      })
    )

    // エラーイベント
    unlisteners.push(
      listen<{ error: string }>('p2p://error', (event) => {
        console.error('P2P error:', event.payload.error)
        useP2PStore.getState().clearError()
      })
    )

    // クリーンアップ
    return () => {
      unlisteners.forEach(async (unlisten) => {
        const fn = await unlisten
        fn()
      })
    }
  }, [addMessage, updatePeer, removePeer, refreshStatus])
}