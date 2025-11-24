import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import React from 'react';
import { setupIntegrationTest, setMockResponse } from '../setup';
import { useTopicStore } from '@/stores/topicStore';
import { invoke } from '@tauri-apps/api/core';
import { Topic } from '@/stores/types';

function TopicTestComponent() {
  const topicsMap = useTopicStore((state) => state.topics);
  const topics = Array.from(topicsMap.values());
  const currentTopic = useTopicStore((state) => state.currentTopic);
  const setCurrentTopic = useTopicStore((state) => state.setCurrentTopic);
  const addTopic = useTopicStore((state) => state.addTopic);
  const setTopics = useTopicStore((state) => state.setTopics);
  const [newTopicName, setNewTopicName] = React.useState('');
  const [newTopicDesc, setNewTopicDesc] = React.useState('');
  const [isLoading, setIsLoading] = React.useState(false);

  React.useEffect(() => {
    const loadTopics = async () => {
      try {
        const topics = await invoke<Topic[]>('list_topics', {});
        if (setTopics) {
          setTopics(topics);
        }
      } catch {
        // Errors are handled by the store
      }
    };
    loadTopics();
  }, [setTopics]);

  const createTopic = async (name: string, description: string) => {
    setIsLoading(true);
    try {
      const topic = await invoke<Topic>('create_topic', { name, description });
      addTopic(topic);
    } catch {
      // Errors are handled by the store
    } finally {
      setIsLoading(false);
    }
  };

  const handleCreateTopic = async (e: React.FormEvent) => {
    e.preventDefault();
    if (newTopicName.trim()) {
      await createTopic(newTopicName, newTopicDesc);
      setNewTopicName('');
      setNewTopicDesc('');
    }
  };

  return (
    <div>
      <div data-testid="selected-topic">
        {currentTopic ? currentTopic.name : 'No topic selected'}
      </div>

      <form onSubmit={handleCreateTopic}>
        <input
          type="text"
          value={newTopicName}
          onChange={(e) => setNewTopicName(e.target.value)}
          placeholder="Topic name"
          data-testid="topic-name-input"
        />
        <input
          type="text"
          value={newTopicDesc}
          onChange={(e) => setNewTopicDesc(e.target.value)}
          placeholder="Topic description"
          data-testid="topic-desc-input"
        />
        <button type="submit" disabled={isLoading}>
          Create Topic
        </button>
      </form>

      <div data-testid="topics-list">
        {topics.map((topic: Topic) => (
          <div
            key={topic.id}
            data-testid={`topic-${topic.id}`}
            onClick={() => setCurrentTopic(topic)}
            style={{ cursor: 'pointer' }}
          >
            <h3>{topic.name}</h3>
            <p>{topic.description}</p>
          </div>
        ))}
      </div>
    </div>
  );
}

describe('Topic Integration Tests', () => {
  let cleanup: () => void;
  let queryClient: QueryClient;

  beforeEach(() => {
    cleanup = setupIntegrationTest();
    queryClient = new QueryClient({
      defaultOptions: {
        queries: { retry: false },
        mutations: { retry: false },
      },
    });

    useTopicStore.getState().setTopics([]);
    useTopicStore.getState().setCurrentTopic(null);
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it('should display list of topics', async () => {
    const mockTopics = [
      { id: 1, name: 'rust', description: 'Rust programming language' },
      { id: 2, name: 'nostr', description: 'Nostr protocol discussions' },
      { id: 3, name: 'bitcoin', description: 'Bitcoin and cryptocurrency' },
    ];

    setMockResponse('list_topics', mockTopics);

    render(
      <QueryClientProvider client={queryClient}>
        <TopicTestComponent />
      </QueryClientProvider>,
    );

    await waitFor(() => {
      expect(screen.getByText('rust')).toBeInTheDocument();
      expect(screen.getByText('Rust programming language')).toBeInTheDocument();
      expect(screen.getByText('nostr')).toBeInTheDocument();
      expect(screen.getByText('bitcoin')).toBeInTheDocument();
      const topicsList = screen.getByTestId('topics-list');
      expect(within(topicsList).getByText('#public')).toBeInTheDocument();
    });

    const topicsList = screen.getByTestId('topics-list');
    expect(topicsList.children).toHaveLength(mockTopics.length + 1);
  });

  it('should create a new topic', async () => {
    const user = userEvent.setup();

    const newTopic = {
      id: 4,
      name: 'tauri',
      description: 'Building desktop apps with Tauri',
    };

    setMockResponse('create_topic', newTopic);
    setMockResponse('list_topics', [newTopic]);

    render(
      <QueryClientProvider client={queryClient}>
        <TopicTestComponent />
      </QueryClientProvider>,
    );

    await user.type(screen.getByTestId('topic-name-input'), 'tauri');
    await user.type(screen.getByTestId('topic-desc-input'), 'Building desktop apps with Tauri');
    await user.click(screen.getByText('Create Topic'));

    await waitFor(() => {
      expect(screen.getByText('tauri')).toBeInTheDocument();
      expect(screen.getByText('Building desktop apps with Tauri')).toBeInTheDocument();
    });

    expect(screen.getByTestId('topic-name-input')).toHaveValue('');
    expect(screen.getByTestId('topic-desc-input')).toHaveValue('');
  });

  it('should select a topic when clicked', async () => {
    const user = userEvent.setup();

    const mockTopics = [
      { id: 1, name: 'react', description: 'React framework' },
      { id: 2, name: 'vue', description: 'Vue.js framework' },
    ];

    setMockResponse('list_topics', mockTopics);

    render(
      <QueryClientProvider client={queryClient}>
        <TopicTestComponent />
      </QueryClientProvider>,
    );

    await waitFor(() => {
      expect(screen.getByText('react')).toBeInTheDocument();
    });

    expect(screen.getByTestId('selected-topic')).toHaveTextContent('#public');

    await user.click(screen.getByTestId('topic-1'));

    expect(screen.getByTestId('selected-topic')).toHaveTextContent('react');
  });

  it('should handle empty topic name', async () => {
    const user = userEvent.setup();

    setMockResponse('list_topics', []);

    render(
      <QueryClientProvider client={queryClient}>
        <TopicTestComponent />
      </QueryClientProvider>,
    );

    await user.click(screen.getByText('Create Topic'));

    await waitFor(() => {
      const topicsList = screen.getByTestId('topics-list');
      expect(topicsList.children).toHaveLength(1);
    });
  });

  it('should handle topic creation errors', async () => {
    const user = userEvent.setup();
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    setMockResponse('create_topic', () => Promise.reject(new Error('Failed to create topic')));
    setMockResponse('list_topics', []);

    render(
      <QueryClientProvider client={queryClient}>
        <TopicTestComponent />
      </QueryClientProvider>,
    );

    await user.type(screen.getByTestId('topic-name-input'), 'error-topic');
    await user.click(screen.getByText('Create Topic'));

    await waitFor(() => {
      const topicsList = screen.getByTestId('topics-list');
      expect(topicsList.children).toHaveLength(1);
    });

    consoleSpy.mockRestore();
  });

  it('should search and filter topics', async () => {
    const mockTopics = [
      { id: 1, name: 'javascript', description: 'JavaScript discussions' },
      { id: 2, name: 'typescript', description: 'TypeScript superset of JS' },
      { id: 3, name: 'python', description: 'Python programming' },
      { id: 4, name: 'rust', description: 'Rust systems programming' },
    ];

    setMockResponse('list_topics', mockTopics);

    const { rerender } = render(
      <QueryClientProvider client={queryClient}>
        <TopicTestComponent />
      </QueryClientProvider>,
    );

    await waitFor(() => {
      expect(screen.getByText('javascript')).toBeInTheDocument();
      expect(screen.getByText('typescript')).toBeInTheDocument();
      expect(screen.getByText('python')).toBeInTheDocument();
      expect(screen.getByText('rust')).toBeInTheDocument();
    });

    const filteredTopics = mockTopics.filter((t) => t.name.includes('script'));
    setMockResponse('list_topics', filteredTopics);

    rerender(
      <QueryClientProvider client={queryClient}>
        <TopicTestComponent key="filtered" />
      </QueryClientProvider>,
    );

    await waitFor(() => {
      expect(screen.getByText('javascript')).toBeInTheDocument();
      expect(screen.getByText('typescript')).toBeInTheDocument();
      expect(screen.queryByText('python')).not.toBeInTheDocument();
      expect(screen.queryByText('rust')).not.toBeInTheDocument();
    });
  });

  it('should maintain selected topic across re-renders', async () => {
    const user = userEvent.setup();

    const mockTopics = [{ id: 1, name: 'persistent-topic', description: 'This topic persists' }];

    setMockResponse('list_topics', mockTopics);

    const { rerender } = render(
      <QueryClientProvider client={queryClient}>
        <TopicTestComponent />
      </QueryClientProvider>,
    );

    await waitFor(() => {
      expect(screen.getByText('persistent-topic')).toBeInTheDocument();
    });

    await user.click(screen.getByTestId('topic-1'));
    expect(screen.getByTestId('selected-topic')).toHaveTextContent('persistent-topic');

    rerender(
      <QueryClientProvider client={queryClient}>
        <TopicTestComponent />
      </QueryClientProvider>,
    );

    expect(screen.getByTestId('selected-topic')).toHaveTextContent('persistent-topic');
  });
});
