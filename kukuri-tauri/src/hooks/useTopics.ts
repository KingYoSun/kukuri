import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { useTopicStore } from '@/stores'
import type { Topic } from '@/stores'

// 仮のAPI関数（後でTauriコマンドに置き換え）
const fetchTopics = async (): Promise<Topic[]> => {
  // TODO: Tauriバックエンドから取得
  return [
    {
      id: 'tech',
      name: 'technology',
      description: '技術全般について議論するトピック',
      tags: ['tech', 'programming', 'ai'],
      memberCount: 1234,
      lastActive: Date.now(),
    },
    {
      id: 'nostr',
      name: 'nostr',
      description: 'Nostrプロトコルについて',
      tags: ['nostr', 'decentralized', 'social'],
      memberCount: 456,
      lastActive: Date.now(),
    },
  ]
}

const joinTopic = async (topicId: string): Promise<void> => {
  // TODO: Tauriバックエンドに参加をリクエスト
  console.log('Joining topic:', topicId)
}

const leaveTopic = async (topicId: string): Promise<void> => {
  // TODO: Tauriバックエンドに退出をリクエスト
  console.log('Leaving topic:', topicId)
}

export const useTopics = () => {
  const { setTopics } = useTopicStore()

  return useQuery({
    queryKey: ['topics'],
    queryFn: async () => {
      const topics = await fetchTopics()
      setTopics(topics)
      return topics
    },
  })
}

export const useJoinTopic = () => {
  const queryClient = useQueryClient()
  const { joinTopic: joinTopicStore } = useTopicStore()

  return useMutation({
    mutationFn: joinTopic,
    onSuccess: (_, topicId) => {
      joinTopicStore(topicId)
      queryClient.invalidateQueries({ queryKey: ['topics'] })
    },
  })
}

export const useLeaveTopic = () => {
  const queryClient = useQueryClient()
  const { leaveTopic: leaveTopicStore } = useTopicStore()

  return useMutation({
    mutationFn: leaveTopic,
    onSuccess: (_, topicId) => {
      leaveTopicStore(topicId)
      queryClient.invalidateQueries({ queryKey: ['topics'] })
    },
  })
}