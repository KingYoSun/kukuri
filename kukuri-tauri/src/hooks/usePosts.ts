import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { usePostStore } from '@/stores';
import type { Post } from '@/stores';

// 仮のAPI関数（後でTauriコマンドに置き換え）
const fetchPostsByTopic = async (topicId: string): Promise<Post[]> => {
  // TODO: Tauriバックエンドから取得
  return [
    {
      id: '1',
      content: 'Nostrプロトコルを使った分散型SNSの可能性について考えています。',
      author: {
        id: 'user1',
        pubkey: 'pubkey1',
        npub: 'npub1...',
        name: 'ユーザー1',
        displayName: 'ユーザー1',
        picture: '',
        about: '',
        nip05: '',
      },
      topicId,
      created_at: Math.floor(Date.now() / 1000) - 7200,
      tags: [],
      likes: 10,
      replies: [],
    },
    {
      id: '2',
      content: 'kukuriの開発進捗：P2P通信レイヤーの実装が完了しました！',
      author: {
        id: 'user2',
        pubkey: 'pubkey2',
        npub: 'npub2...',
        name: 'ユーザー2',
        displayName: 'ユーザー2',
        picture: '',
        about: '',
        nip05: '',
      },
      topicId,
      created_at: Math.floor(Date.now() / 1000) - 14400,
      tags: [],
      likes: 25,
      replies: [],
    },
  ];
};

const createPost = async (postData: { content: string; topicId: string }): Promise<Post> => {
  // TODO: Tauriバックエンドに投稿を送信
  return {
    id: Date.now().toString(),
    content: postData.content,
    author: {
      id: 'currentUser',
      pubkey: 'currentPubkey',
      npub: 'currentNpub',
      name: '現在のユーザー',
      displayName: '現在のユーザー',
      picture: '',
      about: '',
      nip05: '',
    },
    topicId: postData.topicId,
    created_at: Math.floor(Date.now() / 1000),
    tags: [],
    likes: 0,
    replies: [],
  };
};

export const usePostsByTopic = (topicId: string) => {
  const { setPosts } = usePostStore();

  return useQuery({
    queryKey: ['posts', topicId],
    queryFn: async () => {
      const posts = await fetchPostsByTopic(topicId);
      setPosts(posts);
      return posts;
    },
    enabled: !!topicId,
  });
};

export const useCreatePost = () => {
  const queryClient = useQueryClient();
  const { addPost } = usePostStore();

  return useMutation({
    mutationFn: createPost,
    onSuccess: (newPost) => {
      addPost(newPost);
      queryClient.invalidateQueries({ queryKey: ['posts', newPost.topicId] });
    },
  });
};
