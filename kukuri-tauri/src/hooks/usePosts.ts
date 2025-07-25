import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { usePostStore } from '@/stores'
import type { Post } from '@/stores'

// 仮のAPI関数（後でTauriコマンドに置き換え）
const fetchPostsByTopic = async (topicId: string): Promise<Post[]> => {
  // TODO: Tauriバックエンドから取得
  return [
    {
      id: '1',
      pubkey: 'npub1...',
      content: 'Nostrプロトコルを使った分散型SNSの可能性について考えています。',
      topicId,
      created_at: Math.floor(Date.now() / 1000) - 7200,
      tags: [],
    },
    {
      id: '2',
      pubkey: 'npub2...',
      content: 'kukuriの開発進捗：P2P通信レイヤーの実装が完了しました！',
      topicId,
      created_at: Math.floor(Date.now() / 1000) - 14400,
      tags: [],
    },
  ]
}

const createPost = async (post: Omit<Post, 'id' | 'created_at'>): Promise<Post> => {
  // TODO: Tauriバックエンドに投稿を送信
  return {
    ...post,
    id: Date.now().toString(),
    created_at: Math.floor(Date.now() / 1000),
  }
}

export const usePostsByTopic = (topicId: string) => {
  const { setPosts } = usePostStore()

  return useQuery({
    queryKey: ['posts', topicId],
    queryFn: async () => {
      const posts = await fetchPostsByTopic(topicId)
      setPosts(posts)
      return posts
    },
    enabled: !!topicId,
  })
}

export const useCreatePost = () => {
  const queryClient = useQueryClient()
  const { addPost } = usePostStore()

  return useMutation({
    mutationFn: createPost,
    onSuccess: (newPost) => {
      addPost(newPost)
      queryClient.invalidateQueries({ queryKey: ['posts', newPost.topicId] })
    },
  })
}