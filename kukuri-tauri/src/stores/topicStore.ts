import { create } from 'zustand';

import type { TopicState, Topic } from './types';
import { TauriApi } from '@/lib/api/tauri';
import { errorHandler } from '@/lib/errorHandler';
import { subscribeToTopic as nostrSubscribe } from '@/lib/api/nostr';
import { useOfflineStore } from './offlineStore';
import { OfflineActionType, EntityType } from '@/types/offline';
import { withPersist } from './utils/persistHelpers';
import { createTopicPersistConfig } from './config/persist';

interface TopicStore extends TopicState {
  setTopics: (topics: Topic[]) => void;
  fetchTopics: () => Promise<void>;
  addTopic: (topic: Topic) => void;
  createTopic: (name: string, description: string) => Promise<Topic>;
  updateTopic: (id: string, update: Partial<Topic>) => void;
  updateTopicRemote: (id: string, name: string, description: string) => Promise<void>;
  removeTopic: (id: string) => void;
  deleteTopicRemote: (id: string) => Promise<void>;
  setCurrentTopic: (topic: Topic | null) => void;
  joinTopic: (topicId: string) => Promise<void>;
  leaveTopic: (topicId: string) => Promise<void>;
  updateTopicPostCount: (topicId: string, delta: number) => void;
}

export const useTopicStore = create<TopicStore>()(
  withPersist<TopicStore>(
    (set) => ({
      topics: new Map(),
      currentTopic: null,
      joinedTopics: [],

      setTopics: (topics: Topic[]) =>
        set({
          topics: new Map(topics.map((t) => [t.id, t])),
        }),

      fetchTopics: async () => {
        try {
          const apiTopics = await TauriApi.getTopics();
          if (!apiTopics) {
            set({ topics: new Map() });
            return;
          }
          const topics: Topic[] = apiTopics.map((t) => ({
            id: t.id,
            name: t.name,
            description: t.description,
            createdAt: new Date(t.created_at * 1000),
            memberCount: 0,
            postCount: 0,
            isActive: true,
            tags: [],
          }));
          set({
            topics: new Map(topics.map((t) => [t.id, t])),
          });
        } catch (error) {
          errorHandler.log('Failed to fetch topics', error, {
            context: 'TopicStore.fetchTopics',
            showToast: true,
            toastTitle: 'トピックの取得に失敗しました',
          });
          throw error;
        }
      },

      addTopic: (topic: Topic) =>
        set((state) => {
          const newTopics = new Map(state.topics);
          newTopics.set(topic.id, topic);
          return { topics: newTopics };
        }),

      createTopic: async (name: string, description: string) => {
        try {
          const apiTopic = await TauriApi.createTopic({ name, description });
          const topic: Topic = {
            id: apiTopic.id,
            name: apiTopic.name,
            description: apiTopic.description,
            createdAt: new Date(apiTopic.created_at * 1000),
            memberCount: 0,
            postCount: 0,
            isActive: true,
            tags: [],
          };
          set((state) => {
            const newTopics = new Map(state.topics);
            newTopics.set(topic.id, topic);
            return { topics: newTopics };
          });
          return topic;
        } catch (error) {
          errorHandler.log('Failed to create topic', error, {
            context: 'TopicStore.createTopic',
            showToast: true,
            toastTitle: 'トピックの作成に失敗しました',
          });
          throw error;
        }
      },

      updateTopic: (id: string, update: Partial<Topic>) =>
        set((state) => {
          const newTopics = new Map(state.topics);
          const existing = newTopics.get(id);
          if (existing) {
            newTopics.set(id, { ...existing, ...update });
          }
          return { topics: newTopics };
        }),

      updateTopicRemote: async (id: string, name: string, description: string) => {
        try {
          const apiTopic = await TauriApi.updateTopic({ id, name, description });
          set((state) => {
            const newTopics = new Map(state.topics);
            const existing = newTopics.get(id);
            if (existing) {
              newTopics.set(id, {
                ...existing,
                name: apiTopic.name,
                description: apiTopic.description,
              });
            }
            return { topics: newTopics };
          });
        } catch (error) {
          errorHandler.log('Failed to update topic', error, {
            context: 'TopicStore.updateTopicRemote',
            showToast: true,
            toastTitle: 'トピックの更新に失敗しました',
          });
          throw error;
        }
      },

      removeTopic: (id: string) =>
        set((state) => {
          const newTopics = new Map(state.topics);
          newTopics.delete(id);
          return {
            topics: newTopics,
            currentTopic: state.currentTopic?.id === id ? null : state.currentTopic,
          };
        }),

      deleteTopicRemote: async (id: string) => {
        try {
          await TauriApi.deleteTopic(id);
          set((state) => {
            const newTopics = new Map(state.topics);
            newTopics.delete(id);
            return {
              topics: newTopics,
              currentTopic: state.currentTopic?.id === id ? null : state.currentTopic,
            };
          });
        } catch (error) {
          errorHandler.log('Failed to delete topic', error, {
            context: 'TopicStore.deleteTopicRemote',
            showToast: true,
            toastTitle: 'トピックの削除に失敗しました',
          });
          throw error;
        }
      },

      setCurrentTopic: (topic: Topic | null) => set({ currentTopic: topic }),

      joinTopic: async (topicId: string) => {
        // 既に参加している場合は何もしない
        const currentState = useTopicStore.getState();
        if (currentState.joinedTopics.includes(topicId)) {
          return;
        }

        const offlineStore = useOfflineStore.getState();
        const isOnline = offlineStore.isOnline;

        // 先にUIを更新（楽観的UI更新）
        set((state) => ({
          joinedTopics: [...new Set([...state.joinedTopics, topicId])],
        }));

        // オフラインの場合、アクションを保存して後で同期
        if (!isOnline) {
          const userPubkey = localStorage.getItem('currentUserPubkey') || 'unknown';
          await offlineStore.saveOfflineAction({
            userPubkey,
            actionType: OfflineActionType.JOIN_TOPIC,
            entityType: EntityType.TOPIC,
            entityId: topicId,
            data: JSON.stringify({ topicId }),
          });
          return;
        }

        try {
          // バックエンドのサービス層経由で参加処理を実行（P2P join + DB更新）
          await TauriApi.joinTopic(topicId);

          // P2P接続が安定した後にNostrサブスクリプションを開始
          // リレー接続が無効化されている場合でも、将来的な互換性のために呼び出しは維持
          setTimeout(() => {
            nostrSubscribe(topicId).catch((error) => {
              errorHandler.log('Failed to subscribe to Nostr topic', error, {
                context: 'TopicStore.joinTopic.nostrSubscribe',
                showToast: false, // P2Pが成功していればエラーは表示しない
              });
            });
          }, 500); // 500msの遅延
        } catch (error) {
          // エラーが発生した場合は状態を元に戻す
          set((state) => ({
            joinedTopics: state.joinedTopics.filter((id) => id !== topicId),
          }));
          errorHandler.log('Failed to join topic', error, {
            context: 'TopicStore.joinTopic',
            showToast: true,
            toastTitle: 'トピックへの参加に失敗しました',
          });
          throw error;
        }
      },

      leaveTopic: async (topicId: string) => {
        // 参加していない場合は何もしない
        const currentState = useTopicStore.getState();
        if (!currentState.joinedTopics.includes(topicId)) {
          return;
        }

        const offlineStore = useOfflineStore.getState();
        const isOnline = offlineStore.isOnline;

        // 先にUIを更新（楽観的UI更新）
        set((state) => ({
          joinedTopics: state.joinedTopics.filter((id) => id !== topicId),
          currentTopic: state.currentTopic?.id === topicId ? null : state.currentTopic,
        }));

        // オフラインの場合、アクションを保存して後で同期
        if (!isOnline) {
          const userPubkey = localStorage.getItem('currentUserPubkey') || 'unknown';
          await offlineStore.saveOfflineAction({
            userPubkey,
            actionType: OfflineActionType.LEAVE_TOPIC,
            entityType: EntityType.TOPIC,
            entityId: topicId,
            data: JSON.stringify({ topicId }),
          });
          return;
        }

        try {
          // バックエンドのサービス層経由で離脱処理を実行（P2P leave + DB更新）
          await TauriApi.leaveTopic(topicId);
        } catch (error) {
          // エラーが発生した場合は状態を元に戻す
          set((state) => ({
            joinedTopics: [...new Set([...state.joinedTopics, topicId])],
          }));
          errorHandler.log('Failed to leave topic', error, {
            context: 'TopicStore.leaveTopic',
            showToast: true,
            toastTitle: 'トピックからの離脱に失敗しました',
          });
          throw error;
        }
      },

      updateTopicPostCount: (topicId: string, delta: number) =>
        set((state) => {
          const newTopics = new Map(state.topics);
          const topic = newTopics.get(topicId);
          if (topic) {
            newTopics.set(topicId, {
              ...topic,
              postCount: topic.postCount + delta,
            });
          }
          return { topics: newTopics };
        }),
    }),
    createTopicPersistConfig<TopicStore>(),
  ),
);
