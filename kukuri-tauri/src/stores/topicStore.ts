import { create } from 'zustand'
import { persist, createJSONStorage } from 'zustand/middleware'
import type { TopicState, Topic } from './types'

interface TopicStore extends TopicState {
  setTopics: (topics: Topic[]) => void
  addTopic: (topic: Topic) => void
  updateTopic: (id: string, update: Partial<Topic>) => void
  removeTopic: (id: string) => void
  setCurrentTopic: (topic: Topic | null) => void
  joinTopic: (topicId: string) => void
  leaveTopic: (topicId: string) => void
}

export const useTopicStore = create<TopicStore>()(
  persist(
    (set) => ({
      topics: new Map(),
      currentTopic: null,
      joinedTopics: [],

      setTopics: (topics: Topic[]) => 
        set({
          topics: new Map(topics.map(t => [t.id, t]))
        }),

      addTopic: (topic: Topic) =>
        set((state) => {
          const newTopics = new Map(state.topics)
          newTopics.set(topic.id, topic)
          return { topics: newTopics }
        }),

      updateTopic: (id: string, update: Partial<Topic>) =>
        set((state) => {
          const newTopics = new Map(state.topics)
          const existing = newTopics.get(id)
          if (existing) {
            newTopics.set(id, { ...existing, ...update })
          }
          return { topics: newTopics }
        }),

      removeTopic: (id: string) =>
        set((state) => {
          const newTopics = new Map(state.topics)
          newTopics.delete(id)
          return { 
            topics: newTopics,
            currentTopic: state.currentTopic?.id === id ? null : state.currentTopic
          }
        }),

      setCurrentTopic: (topic: Topic | null) =>
        set({ currentTopic: topic }),

      joinTopic: (topicId: string) =>
        set((state) => ({
          joinedTopics: [...new Set([...state.joinedTopics, topicId])]
        })),

      leaveTopic: (topicId: string) =>
        set((state) => ({
          joinedTopics: state.joinedTopics.filter(id => id !== topicId),
          currentTopic: state.currentTopic?.id === topicId ? null : state.currentTopic
        }))
    }),
    {
      name: 'topic-storage',
      storage: createJSONStorage(() => localStorage),
      partialize: (state) => ({
        joinedTopics: state.joinedTopics
      })
    }
  )
)