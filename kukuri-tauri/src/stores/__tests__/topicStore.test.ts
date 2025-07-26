import { describe, it, expect, beforeEach } from 'vitest';
import { useTopicStore } from '../topicStore';
import type { Topic } from '../types';

describe('topicStore', () => {
  const mockTopic1: Topic = {
    id: 'topic1',
    name: 'テストトピック1',
    description: '説明1',
    tags: ['tag1'],
    memberCount: 100,
    postCount: 50,
    lastActive: Date.now(),
    isActive: true,
    createdAt: new Date(),
  };

  const mockTopic2: Topic = {
    id: 'topic2',
    name: 'テストトピック2',
    description: '説明2',
    tags: ['tag2'],
    memberCount: 200,
    postCount: 100,
    lastActive: Date.now(),
    isActive: true,
    createdAt: new Date(),
  };

  beforeEach(() => {
    useTopicStore.setState({
      topics: new Map(),
      currentTopic: null,
      joinedTopics: [],
    });
  });

  it('初期状態が正しく設定されていること', () => {
    const state = useTopicStore.getState();
    expect(state.topics.size).toBe(0);
    expect(state.currentTopic).toBeNull();
    expect(state.joinedTopics).toEqual([]);
  });

  it('setTopicsメソッドが正しく動作すること', () => {
    useTopicStore.getState().setTopics([mockTopic1, mockTopic2]);

    const state = useTopicStore.getState();
    expect(state.topics.size).toBe(2);
    expect(state.topics.get('topic1')).toEqual(mockTopic1);
    expect(state.topics.get('topic2')).toEqual(mockTopic2);
  });

  it('addTopicメソッドが正しく動作すること', () => {
    useTopicStore.getState().addTopic(mockTopic1);

    const state = useTopicStore.getState();
    expect(state.topics.size).toBe(1);
    expect(state.topics.get('topic1')).toEqual(mockTopic1);
  });

  it('updateTopicメソッドが正しく動作すること', () => {
    useTopicStore.setState({
      topics: new Map([['topic1', mockTopic1]]),
    });

    useTopicStore.getState().updateTopic('topic1', { memberCount: 150 });

    const state = useTopicStore.getState();
    expect(state.topics.get('topic1')?.memberCount).toBe(150);
  });

  it('removeTopicメソッドが正しく動作すること', () => {
    useTopicStore.setState({
      topics: new Map([
        ['topic1', mockTopic1],
        ['topic2', mockTopic2],
      ]),
      currentTopic: mockTopic1,
    });

    useTopicStore.getState().removeTopic('topic1');

    const state = useTopicStore.getState();
    expect(state.topics.size).toBe(1);
    expect(state.topics.has('topic1')).toBe(false);
    expect(state.currentTopic).toBeNull();
  });

  it('setCurrentTopicメソッドが正しく動作すること', () => {
    useTopicStore.getState().setCurrentTopic(mockTopic1);

    expect(useTopicStore.getState().currentTopic).toEqual(mockTopic1);
  });

  it('joinTopicメソッドが正しく動作すること', () => {
    const { joinTopic } = useTopicStore.getState();
    joinTopic('topic1');
    joinTopic('topic2');
    joinTopic('topic1'); // 重複

    const state = useTopicStore.getState();
    expect(state.joinedTopics).toEqual(['topic1', 'topic2']);
  });

  it('leaveTopicメソッドが正しく動作すること', () => {
    useTopicStore.setState({
      joinedTopics: ['topic1', 'topic2'],
      currentTopic: mockTopic1,
    });

    useTopicStore.getState().leaveTopic('topic1');

    const state = useTopicStore.getState();
    expect(state.joinedTopics).toEqual(['topic2']);
    expect(state.currentTopic).toBeNull();
  });
});
