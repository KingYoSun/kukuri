import { create } from 'zustand';
import { persist, createJSONStorage } from 'zustand/middleware';
import type { TopicState, Topic } from './types';
import { TauriApi } from '@/lib/api/tauri';

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
  joinTopic: (topicId: string) => void;
  leaveTopic: (topicId: string) => void;
}

export const useTopicStore = create<TopicStore>()(
  persist(
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
          console.error('Failed to fetch topics:', error);
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
          console.error('Failed to create topic:', error);
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
          console.error('Failed to update topic:', error);
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
          console.error('Failed to delete topic:', error);
          throw error;
        }
      },

      setCurrentTopic: (topic: Topic | null) => set({ currentTopic: topic }),

      joinTopic: (topicId: string) =>
        set((state) => ({
          joinedTopics: [...new Set([...state.joinedTopics, topicId])],
        })),

      leaveTopic: (topicId: string) =>
        set((state) => ({
          joinedTopics: state.joinedTopics.filter((id) => id !== topicId),
          currentTopic: state.currentTopic?.id === topicId ? null : state.currentTopic,
        })),
    }),
    {
      name: 'topic-storage',
      storage: createJSONStorage(() => localStorage),
      partialize: (state) => ({
        joinedTopics: state.joinedTopics,
      }),
    },
  ),
);
