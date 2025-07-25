import { describe, it, expect, beforeEach } from 'vitest'
import { renderHook, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { useTopics, useJoinTopic, useLeaveTopic } from '../useTopics'
import { useTopicStore } from '@/stores'
import { ReactNode } from 'react'

const createWrapper = () => {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  })
  
  return ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={queryClient}>
      {children}
    </QueryClientProvider>
  )
}

describe('useTopics hooks', () => {
  beforeEach(() => {
    useTopicStore.setState({
      topics: new Map(),
      currentTopic: null,
      joinedTopics: [],
    })
  })

  describe('useTopics', () => {
    it('トピック取得成功時にtopicStoreが更新されること', async () => {
      const { result } = renderHook(() => useTopics(), {
        wrapper: createWrapper(),
      })

      await waitFor(() => {
        expect(result.current.isSuccess).toBe(true)
      })

      const state = useTopicStore.getState()
      expect(state.topics.size).toBeGreaterThan(0)
      expect(state.topics.has('tech')).toBe(true)
      expect(state.topics.has('nostr')).toBe(true)
    })
  })

  describe('useJoinTopic', () => {
    it('トピック参加成功時にjoinedTopicsが更新されること', async () => {
      const { result } = renderHook(() => useJoinTopic(), {
        wrapper: createWrapper(),
      })

      await result.current.mutateAsync('tech')

      await waitFor(() => {
        const state = useTopicStore.getState()
        expect(state.joinedTopics).toContain('tech')
      })
    })
  })

  describe('useLeaveTopic', () => {
    it('トピック退出成功時にjoinedTopicsから削除されること', async () => {
      useTopicStore.setState({
        joinedTopics: ['tech', 'nostr'],
      })

      const { result } = renderHook(() => useLeaveTopic(), {
        wrapper: createWrapper(),
      })

      await result.current.mutateAsync('tech')

      await waitFor(() => {
        const state = useTopicStore.getState()
        expect(state.joinedTopics).not.toContain('tech')
        expect(state.joinedTopics).toContain('nostr')
      })
    })
  })
})