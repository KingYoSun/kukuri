import { create } from 'zustand';

import { DEFAULT_PUBLIC_TOPIC_ID } from '@/constants/topics';
import { subscribeToTopic as nostrSubscribe } from '@/lib/api/nostr';
import { TauriApi, type PendingTopic } from '@/lib/api/tauri';
import { errorHandler } from '@/lib/errorHandler';
import { OfflineActionType, EntityType } from '@/types/offline';
import { useComposerStore } from './composerStore';
import { useOfflineStore } from './offlineStore';
import { createTopicPersistConfig } from './config/persist';
import { withPersist } from './utils/persistHelpers';
import type { Topic, TopicState } from './types';

interface TopicStore extends TopicState {
  setTopics: (topics: Topic[]) => void;
  fetchTopics: () => Promise<void>;
  addTopic: (topic: Topic) => void;
  createTopic: (name: string, description: string) => Promise<Topic>;
  queueTopicCreation: (name: string, description: string) => Promise<PendingTopic>;
  updateTopic: (id: string, update: Partial<Topic>) => void;
  updateTopicRemote: (id: string, name: string, description: string) => Promise<void>;
  removeTopic: (id: string) => void;
  deleteTopicRemote: (id: string) => Promise<void>;
  setCurrentTopic: (topic: Topic | null) => void;
  joinTopic: (topicId: string) => Promise<void>;
  leaveTopic: (topicId: string) => Promise<void>;
  updateTopicPostCount: (topicId: string, delta: number) => void;
  markTopicRead: (topicId: string) => void;
  handleIncomingTopicMessage: (topicId: string, timestamp: number) => void;
  setPendingTopics: (pending: PendingTopic[]) => void;
  upsertPendingTopic: (pending: PendingTopic) => void;
  removePendingTopic: (pendingId: string) => void;
  refreshPendingTopics: () => Promise<void>;
}

const normalizeTopicId = (topicId: string) =>
  topicId === 'public' ? DEFAULT_PUBLIC_TOPIC_ID : topicId;

const computeTopicCollections = (state: TopicStore, topics: Topic[]) => {
  const nextTopics = new Map(
    topics.map((t) => {
      const normalizedId = normalizeTopicId(t.id);
      return [normalizedId, { ...t, id: normalizedId }];
    }),
  );
  const topicIds = new Set(nextTopics.keys());

  const unread = new Map(
    Array.from(state.topicUnreadCounts.entries()).filter(([id]) => topicIds.has(id)),
  );
  const lastRead = new Map(
    Array.from(state.topicLastReadAt.entries()).filter(([id]) => topicIds.has(id)),
  );

  return {
    topics: nextTopics,
    topicUnreadCounts: unread,
    topicLastReadAt: lastRead,
  };
};

export const useTopicStore = create<TopicStore>()(
  withPersist<TopicStore>((set, get) => {
    const handlePendingTransition = (previous: PendingTopic | undefined, next: PendingTopic) => {
      if (next.status === 'synced' && next.synced_topic_id && previous?.status !== 'synced') {
        useComposerStore.getState().resolvePendingTopic(next.pending_id, next.synced_topic_id);
        void get()
          .fetchTopics()
          .catch((error) => {
            errorHandler.log('Failed to refresh topics after pending sync', error, {
              context: 'TopicStore.handlePendingTransition',
            });
          });
      } else if (next.status === 'failed' && previous?.status !== 'failed') {
        useComposerStore.getState().clearPendingTopicBinding(next.pending_id);
      }
    };

    return {
      topics: new Map(),
      currentTopic: null,
      joinedTopics: [],
      topicUnreadCounts: new Map(),
      topicLastReadAt: new Map(),
      pendingTopics: new Map(),

      setTopics: (topics: Topic[]) => set((state) => computeTopicCollections(state, topics)),

      setPendingTopics: (pendingList: PendingTopic[]) =>
        set((state) => {
          const next = new Map(state.pendingTopics);
          const incoming = new Set(pendingList.map((p) => p.pending_id));
          pendingList.forEach((pending) => {
            const previous = next.get(pending.pending_id);
            next.set(pending.pending_id, pending);
            handlePendingTransition(previous, pending);
          });
          next.forEach((_, key) => {
            if (!incoming.has(key)) {
              next.delete(key);
              useComposerStore.getState().clearPendingTopicBinding(key);
            }
          });
          return { pendingTopics: next };
        }),

      upsertPendingTopic: (pending: PendingTopic) =>
        set((state) => {
          const next = new Map(state.pendingTopics);
          const previous = next.get(pending.pending_id);
          next.set(pending.pending_id, pending);
          handlePendingTransition(previous, pending);
          return { pendingTopics: next };
        }),

      removePendingTopic: (pendingId: string) =>
        set((state) => {
          if (!state.pendingTopics.has(pendingId)) {
            return state;
          }
          const next = new Map(state.pendingTopics);
          next.delete(pendingId);
          useComposerStore.getState().clearPendingTopicBinding(pendingId);
          return { pendingTopics: next };
        }),

      fetchTopics: async () => {
        try {
          const apiTopics = await TauriApi.getTopics();
          if (!apiTopics) {
            set({
              topics: new Map(),
              topicUnreadCounts: new Map(),
              topicLastReadAt: new Map(),
            });
            return;
          }
          const topics: Topic[] = apiTopics.map((t) => ({
            id: normalizeTopicId(t.id),
            name: t.name,
            description: t.description,
            createdAt: new Date(t.created_at * 1000),
            memberCount: 0,
            postCount: 0,
            isActive: true,
            tags: [],
            visibility: t.visibility ?? 'public',
          }));
          set((state) => computeTopicCollections(state, topics));
          const refreshPendingTopics = get().refreshPendingTopics;
          if (typeof refreshPendingTopics === 'function') {
            await refreshPendingTopics();
          }
        } catch (error) {
          errorHandler.log('Failed to fetch topics', error, {
            context: 'TopicStore.fetchTopics',
            showToast: true,
            toastTitle: 'トピックの取得に失敗しました',
          });
          throw error;
        }
      },

      refreshPendingTopics: async () => {
        try {
          const pending = await TauriApi.listPendingTopics();
          get().setPendingTopics(pending);
        } catch (error) {
          errorHandler.log('Failed to load pending topics', error, {
            context: 'TopicStore.refreshPendingTopics',
          });
        }
      },

      addTopic: (topic: Topic) =>
        set((state) => {
          const normalizedId = normalizeTopicId(topic.id);
          const newTopics = new Map(state.topics);
          newTopics.set(normalizedId, { ...topic, id: normalizedId });
          return { topics: newTopics };
        }),

      queueTopicCreation: async (name: string, description: string) => {
        try {
          const response = await TauriApi.enqueueTopicCreation({
            name,
            description,
          });
          set((state) => {
            const next = new Map(state.pendingTopics);
            const pending = response.pending_topic;
            const previous = next.get(pending.pending_id);
            next.set(pending.pending_id, pending);
            handlePendingTransition(previous, pending);
            return { pendingTopics: next };
          });
          useOfflineStore.getState().addPendingAction(response.offline_action);
          return response.pending_topic;
        } catch (error) {
          errorHandler.log('Failed to queue topic creation', error, {
            context: 'TopicStore.queueTopicCreation',
            showToast: true,
            toastTitle: 'トピックの作成予約に失敗しました',
          });
          throw error;
        }
      },

      createTopic: async (name: string, description: string) => {
        try {
          const apiTopic = await TauriApi.createTopic({ name, description });
          const topic: Topic = {
            id: normalizeTopicId(apiTopic.id),
            name: apiTopic.name,
            description: apiTopic.description,
            createdAt: new Date(apiTopic.created_at * 1000),
            memberCount: 0,
            postCount: 0,
            isActive: true,
            tags: [],
            visibility: apiTopic.visibility ?? 'public',
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
          const topicId = normalizeTopicId(id);
          const newTopics = new Map(state.topics);
          const existing = newTopics.get(topicId);
          if (existing) {
            newTopics.set(topicId, { ...existing, ...update, id: topicId });
          }
          return { topics: newTopics };
        }),

      updateTopicRemote: async (id: string, name: string, description: string) => {
        const topicId = normalizeTopicId(id);
        try {
          const apiTopic = await TauriApi.updateTopic({ id: topicId, name, description });
          set((state) => {
            const newTopics = new Map(state.topics);
            const existing = newTopics.get(topicId);
            if (existing) {
              newTopics.set(topicId, {
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
          const topicId = normalizeTopicId(id);
          const newTopics = new Map(state.topics);
          newTopics.delete(topicId);

          const unread = new Map(state.topicUnreadCounts);
          unread.delete(topicId);

          const lastRead = new Map(state.topicLastReadAt);
          lastRead.delete(topicId);

          return {
            topics: newTopics,
            currentTopic: state.currentTopic?.id === topicId ? null : state.currentTopic,
            topicUnreadCounts: unread,
            topicLastReadAt: lastRead,
          };
        }),

      deleteTopicRemote: async (id: string) => {
        const topicId = normalizeTopicId(id);
        try {
          await TauriApi.deleteTopic(topicId);
          set((state) => {
            const newTopics = new Map(state.topics);
            newTopics.delete(topicId);

            const unread = new Map(state.topicUnreadCounts);
            unread.delete(topicId);

            const lastRead = new Map(state.topicLastReadAt);
            lastRead.delete(topicId);

            return {
              topics: newTopics,
              currentTopic: state.currentTopic?.id === topicId ? null : state.currentTopic,
              topicUnreadCounts: unread,
              topicLastReadAt: lastRead,
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

      setCurrentTopic: (topic: Topic | null) =>
        set((state) => {
          if (!topic) {
            return { currentTopic: null };
          }

          const topicId = normalizeTopicId(topic.id);
          const unread = new Map(state.topicUnreadCounts);
          unread.set(topicId, 0);

          const lastRead = new Map(state.topicLastReadAt);
          lastRead.set(topicId, Math.floor(Date.now() / 1000));

          return {
            currentTopic: { ...topic, id: topicId },
            topicUnreadCounts: unread,
            topicLastReadAt: lastRead,
          };
        }),

      joinTopic: async (topicId: string) => {
        const normalizedId = normalizeTopicId(topicId);
        const currentState = useTopicStore.getState();
        if (currentState.joinedTopics.includes(normalizedId)) {
          return;
        }

        const offlineStore = useOfflineStore.getState();
        const isOnline = offlineStore.isOnline;

        set((state) => {
          const joinedTopics = [...new Set([...state.joinedTopics, normalizedId])];
          const unread = new Map(state.topicUnreadCounts);
          unread.set(normalizedId, 0);
          const lastRead = new Map(state.topicLastReadAt);
          lastRead.set(normalizedId, Math.floor(Date.now() / 1000));

          return {
            joinedTopics,
            topicUnreadCounts: unread,
            topicLastReadAt: lastRead,
          };
        });

        if (!isOnline) {
          const userPubkey = localStorage.getItem('currentUserPubkey') || 'unknown';
          await offlineStore.saveOfflineAction({
            userPubkey,
            actionType: OfflineActionType.JOIN_TOPIC,
            entityType: EntityType.TOPIC,
            entityId: normalizedId,
            data: JSON.stringify({ topicId: normalizedId }),
          });
          return;
        }

        try {
          await TauriApi.joinTopic(normalizedId);

          setTimeout(() => {
            nostrSubscribe(normalizedId).catch((error) => {
              errorHandler.log('Failed to subscribe to Nostr topic', error, {
                context: 'TopicStore.joinTopic.nostrSubscribe',
                showToast: false,
              });
            });
          }, 500);
        } catch (error) {
          set((state) => {
            const joinedTopics = state.joinedTopics.filter((id) => id !== normalizedId);
            const unread = new Map(state.topicUnreadCounts);
            unread.delete(normalizedId);
            const lastRead = new Map(state.topicLastReadAt);
            lastRead.delete(normalizedId);
            return {
              joinedTopics,
              topicUnreadCounts: unread,
              topicLastReadAt: lastRead,
            };
          });
          errorHandler.log('Failed to join topic', error, {
            context: 'TopicStore.joinTopic',
            showToast: true,
            toastTitle: 'トピックへの参加に失敗しました',
          });
          throw error;
        }
      },

      leaveTopic: async (topicId: string) => {
        const normalizedId = normalizeTopicId(topicId);
        const currentState = useTopicStore.getState();
        if (!currentState.joinedTopics.includes(normalizedId)) {
          return;
        }

        const offlineStore = useOfflineStore.getState();
        const isOnline = offlineStore.isOnline;

        let previousUnread: number | undefined;
        let previousLastRead: number | undefined;
        set((state) => {
          previousUnread = state.topicUnreadCounts.get(normalizedId);
          previousLastRead = state.topicLastReadAt.get(normalizedId);

          const joinedTopics = state.joinedTopics.filter((id) => id !== normalizedId);
          const unread = new Map(state.topicUnreadCounts);
          unread.delete(normalizedId);
          const lastRead = new Map(state.topicLastReadAt);
          lastRead.delete(normalizedId);

          return {
            joinedTopics,
            currentTopic: state.currentTopic?.id === normalizedId ? null : state.currentTopic,
            topicUnreadCounts: unread,
            topicLastReadAt: lastRead,
          };
        });

        if (!isOnline) {
          const userPubkey = localStorage.getItem('currentUserPubkey') || 'unknown';
          await offlineStore.saveOfflineAction({
            userPubkey,
            actionType: OfflineActionType.LEAVE_TOPIC,
            entityType: EntityType.TOPIC,
            entityId: normalizedId,
            data: JSON.stringify({ topicId: normalizedId }),
          });
          return;
        }

        try {
          await TauriApi.leaveTopic(normalizedId);
        } catch (error) {
          set((state) => {
            const joinedTopics = [...new Set([...state.joinedTopics, normalizedId])];
            const unread = new Map(state.topicUnreadCounts);
            unread.set(normalizedId, previousUnread ?? 0);
            const lastRead = new Map(state.topicLastReadAt);
            if (previousLastRead !== undefined) {
              lastRead.set(normalizedId, previousLastRead);
            }
            return {
              joinedTopics,
              topicUnreadCounts: unread,
              topicLastReadAt: lastRead,
            };
          });
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
          const normalizedId = normalizeTopicId(topicId);
          const newTopics = new Map(state.topics);
          const topic = newTopics.get(normalizedId);
          if (topic) {
            newTopics.set(normalizedId, {
              ...topic,
              postCount: topic.postCount + delta,
            });
          }
          return { topics: newTopics };
        }),

      markTopicRead: (topicId: string) =>
        set((state) => {
          const normalizedId = normalizeTopicId(topicId);
          const unread = new Map(state.topicUnreadCounts);
          unread.set(normalizedId, 0);

          const lastRead = new Map(state.topicLastReadAt);
          lastRead.set(normalizedId, Math.floor(Date.now() / 1000));

          return {
            topicUnreadCounts: unread,
            topicLastReadAt: lastRead,
          };
        }),

      handleIncomingTopicMessage: (topicId: string, timestamp: number) =>
        set((state) => {
          const normalizedId = normalizeTopicId(topicId);
          const unread = new Map(state.topicUnreadCounts);
          const lastRead = new Map(state.topicLastReadAt);
          const normalisedTimestamp =
            timestamp > 1_000_000_000_000 ? Math.floor(timestamp / 1000) : Math.floor(timestamp);

          if (state.currentTopic?.id === normalizedId) {
            unread.set(normalizedId, 0);
            lastRead.set(normalizedId, normalisedTimestamp);
          } else {
            const current = unread.get(normalizedId) ?? 0;
            unread.set(normalizedId, current + 1);
          }

          return {
            topicUnreadCounts: unread,
            topicLastReadAt: lastRead,
          };
        }),
    };
  }, createTopicPersistConfig<TopicStore>()),
);
