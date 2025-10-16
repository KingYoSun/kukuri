import { useEffect, useCallback } from 'react';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { useQueryClient } from '@tanstack/react-query';
import { errorHandler } from '@/lib/errorHandler';
import { usePostStore } from '@/stores/postStore';
import { useTopicStore } from '@/stores/topicStore';
import type { NostrEventPayload } from '@/types/nostr';

/**
 * Nostrイベントリスナーフック
 * Nostrネットワークからのイベントをリアルタイムで受信し、
 * アプリケーションの状態を更新します
 */
export function useNostrEvents() {
  const queryClient = useQueryClient();
  const { incrementLikes } = usePostStore();
  const { updateTopicPostCount } = useTopicStore();

  // 投稿関連イベントの処理
  const handlePostEvent = useCallback(
    (payload: NostrEventPayload) => {
      try {
        // 新規投稿（kind: 1 または kind: 30078）
        if (payload.kind === 1 || payload.kind === 30078) {
          // React Queryのキャッシュを無効化して最新データを取得
          queryClient.invalidateQueries({ queryKey: ['posts'] });

          // トピック投稿の場合、トピックの投稿数を更新
          if (payload.kind === 30078) {
            const topicTag = payload.tags.find((tag) => tag[0] === 't');
            if (topicTag?.[1]) {
              updateTopicPostCount(topicTag[1], 1);
            }
          }

          // リアルタイム更新イベントを発火
          window.dispatchEvent(new Event('realtime-update'));
        }
      } catch (error) {
        errorHandler.log('Failed to handle post event', error, {
          context: 'useNostrEvents.handlePostEvent',
          showToast: false,
        });
      }
    },
    [queryClient, updateTopicPostCount],
  );

  // リアクション関連イベントの処理
  const handleReactionEvent = useCallback(
    (payload: NostrEventPayload) => {
      try {
        // いいね（kind: 7）
        if (payload.kind === 7) {
          const eventIdTag = payload.tags.find((tag) => tag[0] === 'e');
          if (eventIdTag?.[1]) {
            const postId = eventIdTag[1];
            // 楽観的UIアップデート
            incrementLikes(postId);

            // React Queryのキャッシュも更新
            queryClient.setQueryData(
              ['posts'],
              (
                oldData:
                  | {
                      pages?: Array<{
                        posts?: Array<{ id: string; likes: number; [key: string]: unknown }>;
                      }>;
                    }
                  | undefined,
              ) => {
                if (!oldData) return oldData;
                return {
                  ...oldData,
                  pages: oldData.pages?.map((page) => ({
                    ...page,
                    posts: page.posts?.map((post) =>
                      post.id === postId ? { ...post, likes: post.likes + 1 } : post,
                    ),
                  })),
                };
              },
            );
          }
        }
      } catch (error) {
        errorHandler.log('Failed to handle reaction event', error, {
          context: 'useNostrEvents.handleReactionEvent',
          showToast: false,
        });
      }
    },
    [incrementLikes, queryClient],
  );

  // トピック関連イベントの処理
  const handleTopicEvent = useCallback(
    (payload: NostrEventPayload) => {
      try {
        // トピック作成/更新（kind: 30030）
        if (payload.kind === 30030) {
          // React Queryのキャッシュを無効化
          queryClient.invalidateQueries({ queryKey: ['topics'] });
        }
      } catch (error) {
        errorHandler.log('Failed to handle topic event', error, {
          context: 'useNostrEvents.handleTopicEvent',
          showToast: false,
        });
      }
    },
    [queryClient],
  );

  // イベント削除の処理
  const handleDeleteEvent = useCallback(
    (payload: NostrEventPayload) => {
      try {
        // イベント削除（kind: 5）
        if (payload.kind === 5) {
          const deletedEventIds = payload.tags.filter((tag) => tag[0] === 'e').map((tag) => tag[1]);

          if (deletedEventIds.length > 0) {
            // 投稿とトピックの両方のキャッシュを無効化
            queryClient.invalidateQueries({ queryKey: ['posts'] });
            queryClient.invalidateQueries({ queryKey: ['topics'] });
          }
        }
      } catch (error) {
        errorHandler.log('Failed to handle delete event', error, {
          context: 'useNostrEvents.handleDeleteEvent',
          showToast: false,
        });
      }
    },
    [queryClient],
  );

  // メインのイベントハンドラー
  const handleNostrEvent = useCallback(
    (event: { payload: NostrEventPayload }) => {
      const { payload } = event;

      switch (payload.kind) {
        case 1: // テキストノート
        case 30078: // トピック投稿
          handlePostEvent(payload);
          break;
        case 7: // リアクション
          handleReactionEvent(payload);
          break;
        case 30030: // トピック作成/更新
          handleTopicEvent(payload);
          break;
        case 5: // イベント削除
          handleDeleteEvent(payload);
          break;
        default:
          // その他のイベントタイプは現時点では無視
          break;
      }
    },
    [handlePostEvent, handleReactionEvent, handleTopicEvent, handleDeleteEvent],
  );

  useEffect(() => {
    let unlisten: UnlistenFn | null = null;

    const setupListener = async () => {
      try {
        // Nostrイベントリスナーを設定
        unlisten = await listen<NostrEventPayload>('nostr://event', handleNostrEvent);
      } catch (error) {
        errorHandler.log('Failed to setup Nostr event listener', error, {
          context: 'useNostrEvents.setupListener',
          showToast: false,
        });
      }
    };

    setupListener();

    // クリーンアップ
    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, [handleNostrEvent]);
}
