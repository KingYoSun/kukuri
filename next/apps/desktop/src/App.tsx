import { FormEvent, startTransition, useCallback, useEffect, useMemo, useState } from 'react';

import { DesktopApi, PostView, SyncStatus, runtimeApi } from './lib/api';

type AppProps = {
  api?: DesktopApi;
};

const DEFAULT_TOPIC = 'kukuri:topic:demo';
const REFRESH_INTERVAL_MS = 2000;

export function App({ api = runtimeApi }: AppProps) {
  const [trackedTopics, setTrackedTopics] = useState<string[]>([DEFAULT_TOPIC]);
  const [activeTopic, setActiveTopic] = useState(DEFAULT_TOPIC);
  const [topicInput, setTopicInput] = useState('');
  const [composer, setComposer] = useState('');
  const [timelinesByTopic, setTimelinesByTopic] = useState<Record<string, PostView[]>>({
    [DEFAULT_TOPIC]: [],
  });
  const [thread, setThread] = useState<PostView[]>([]);
  const [selectedThread, setSelectedThread] = useState<string | null>(null);
  const [replyTarget, setReplyTarget] = useState<PostView | null>(null);
  const [peerTicket, setPeerTicket] = useState('');
  const [localPeerTicket, setLocalPeerTicket] = useState<string | null>(null);
  const [syncStatus, setSyncStatus] = useState<SyncStatus>({
    connected: false,
    peer_count: 0,
    pending_events: 0,
    subscribed_topics: [],
  });
  const [error, setError] = useState<string | null>(null);

  const headline = useMemo(
    () => (syncStatus.connected ? 'Live over static peers' : 'Local-first shell'),
    [syncStatus.connected]
  );

  const activeTimeline = useMemo(
    () => timelinesByTopic[activeTopic] ?? [],
    [activeTopic, timelinesByTopic]
  );

  const loadTopics = useCallback(async (
    currentTopics: string[],
    currentActiveTopic: string,
    currentThread: string | null
  ) => {
    try {
      const [timelineViews, status, ticket, threadView] = await Promise.all([
        Promise.all(
          currentTopics.map(async (topic) => ({
            topic,
            timeline: await api.listTimeline(topic, null, 50),
          }))
        ),
        api.getSyncStatus(),
        api.getLocalPeerTicket(),
        currentThread
          ? api.listThread(currentActiveTopic, currentThread, null, 50)
          : Promise.resolve(null),
      ]);
      startTransition(() => {
        setTimelinesByTopic(
          Object.fromEntries(
            timelineViews.map(({ topic, timeline }) => [topic, timeline.items])
          )
        );
        setSyncStatus(status);
        setLocalPeerTicket(ticket);
        if (threadView) {
          setThread(threadView.items);
        } else if (!currentThread) {
          setThread([]);
        }
        setError(null);
      });
    } catch (loadError) {
      setError(loadError instanceof Error ? loadError.message : 'failed to load topic');
    }
  }, [api]);

  useEffect(() => {
    let disposed = false;

    const refresh = async () => {
      if (disposed) {
        return;
      }
      await loadTopics(trackedTopics, activeTopic, selectedThread);
    };

    void refresh();
    const intervalId = window.setInterval(() => {
      void refresh();
    }, REFRESH_INTERVAL_MS);

    return () => {
      disposed = true;
      window.clearInterval(intervalId);
    };
  }, [activeTopic, loadTopics, selectedThread, trackedTopics]);

  function clearThreadContext() {
    setSelectedThread(null);
    setThread([]);
    setReplyTarget(null);
  }

  async function handleAddTopic() {
    const nextTopic = topicInput.trim();
    if (!nextTopic) {
      return;
    }
    const nextTopics = trackedTopics.includes(nextTopic)
      ? trackedTopics
      : [...trackedTopics, nextTopic];
    setTrackedTopics(nextTopics);
    setActiveTopic(nextTopic);
    setTopicInput('');
    clearThreadContext();
    await loadTopics(nextTopics, nextTopic, null);
  }

  async function handleSelectTopic(topic: string) {
    setActiveTopic(topic);
    clearThreadContext();
    await loadTopics(trackedTopics, topic, null);
  }

  async function handleRemoveTopic(topic: string) {
    if (trackedTopics.length === 1) {
      return;
    }
    const nextTopics = trackedTopics.filter((value) => value !== topic);
    const nextActiveTopic = activeTopic === topic ? nextTopics[0] : activeTopic;
    setTrackedTopics(nextTopics);
    setActiveTopic(nextActiveTopic);
    clearThreadContext();
    await loadTopics(nextTopics, nextActiveTopic, null);
  }

  async function handlePublish(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!composer.trim()) {
      return;
    }

    try {
      await api.createPost(activeTopic, composer.trim(), replyTarget?.id ?? null);
      setComposer('');
      await loadTopics(trackedTopics, activeTopic, selectedThread);
      if (selectedThread) {
        await openThread(selectedThread);
      }
      setReplyTarget(null);
    } catch (publishError) {
      setError(publishError instanceof Error ? publishError.message : 'failed to publish');
    }
  }

  async function openThread(threadId: string) {
    try {
      const threadView = await api.listThread(activeTopic, threadId, null, 50);
      startTransition(() => {
        setSelectedThread(threadId);
        setThread(threadView.items);
      });
    } catch (threadError) {
      setError(threadError instanceof Error ? threadError.message : 'failed to load thread');
    }
  }

  function beginReply(post: PostView) {
    setReplyTarget(post);
    if (post.root_id) {
      setSelectedThread(post.root_id);
      void openThread(post.root_id);
      return;
    }
    setSelectedThread(post.id);
    void openThread(post.id);
  }

  function clearReply() {
    setReplyTarget(null);
  }

  async function handleImportPeer() {
    if (!peerTicket.trim()) {
      return;
    }
    try {
      await api.importPeerTicket(peerTicket.trim());
      setPeerTicket('');
      await loadTopics(trackedTopics, activeTopic, selectedThread);
    } catch (importError) {
      setError(importError instanceof Error ? importError.message : 'failed to import peer');
    }
  }

  return (
    <main className='shell'>
      <section className='hero'>
        <div>
          <p className='eyebrow'>kukuri rebuild</p>
          <h1>{headline}</h1>
          <p className='lede'>
            topic, timeline, composer, thread, sync status の5要素だけで Linux MVP を成立させる
            desktop shell
          </p>
        </div>
        <label className='field'>
          <span>Add Topic</span>
          <div className='topic-input-row'>
            <input
              value={topicInput}
              onChange={(e) => setTopicInput(e.target.value)}
              placeholder='kukuri:topic:demo'
            />
            <button
              className='button button-secondary'
              type='button'
              onClick={() => void handleAddTopic()}
            >
              Add
            </button>
          </div>
        </label>
      </section>

      <section className='grid'>
        <aside className='panel panel-accent'>
          <h2>Sync Status</h2>
          <dl className='status-grid'>
            <div>
              <dt>Connected</dt>
              <dd>{syncStatus.connected ? 'yes' : 'no'}</dd>
            </div>
            <div>
              <dt>Peers</dt>
              <dd>{syncStatus.peer_count}</dd>
            </div>
            <div>
              <dt>Pending</dt>
              <dd>{syncStatus.pending_events}</dd>
            </div>
          </dl>
          <label className='field'>
            <span>Your Ticket</span>
            <textarea readOnly value={localPeerTicket ?? ''} className='ticket-output' />
          </label>
          <label className='field'>
            <span>Peer Ticket</span>
            <input
              value={peerTicket}
              onChange={(e) => setPeerTicket(e.target.value)}
              placeholder='nodeid@127.0.0.1:7777'
            />
          </label>
          <button className='button button-secondary' onClick={() => void handleImportPeer()}>
            Import Peer
          </button>
          <section className='topic-list'>
            <div className='panel-header'>
              <h3>Tracked Topics</h3>
              <small>{syncStatus.subscribed_topics.length} active</small>
            </div>
            <ul>
              {trackedTopics.map((topic) => (
                <li
                  key={topic}
                  className={
                    topic === activeTopic ? 'topic-item topic-item-active' : 'topic-item'
                  }
                >
                  <button
                    className='topic-link'
                    type='button'
                    onClick={() => void handleSelectTopic(topic)}
                  >
                    {topic}
                  </button>
                  {trackedTopics.length > 1 ? (
                    <button
                      className='topic-remove'
                      type='button'
                      onClick={() => void handleRemoveTopic(topic)}
                    >
                      x
                    </button>
                  ) : null}
                </li>
              ))}
            </ul>
          </section>
        </aside>

        <section className='panel'>
          <div className='panel-header'>
            <h2>Timeline</h2>
            <span className='active-topic-label'>{activeTopic}</span>
            <button
              className='button button-secondary'
              onClick={() => void loadTopics(trackedTopics, activeTopic, selectedThread)}
            >
              Refresh
            </button>
          </div>
          <form className='composer' onSubmit={handlePublish}>
            {replyTarget ? (
              <div className='reply-banner'>
                <div>
                  <strong>Replying</strong>
                  <p>{replyTarget.content}</p>
                </div>
                <button
                  className='button button-secondary'
                  type='button'
                  onClick={clearReply}
                >
                  Clear
                </button>
              </div>
            ) : null}
            <textarea
              value={composer}
              onChange={(e) => setComposer(e.target.value)}
              placeholder={replyTarget ? 'Write a reply' : 'Write a post'}
            />
            <button className='button' type='submit'>
              {replyTarget ? 'Reply' : 'Publish'}
            </button>
          </form>
          <ul className='post-list'>
            {activeTimeline.map((post) => (
              <li key={post.id}>
                <article className='post-card'>
                  <button className='post-link' onClick={() => void openThread(post.root_id ?? post.id)}>
                    <div className='post-meta'>
                      <span>{post.author_npub}</span>
                      <span>{new Date(post.created_at * 1000).toLocaleTimeString('ja-JP')}</span>
                    </div>
                    <strong>{post.content}</strong>
                    <small>{post.note_id}</small>
                    {post.reply_to ? <em className='reply-chip'>Reply</em> : null}
                  </button>
                  <div className='post-actions'>
                    <button
                      className='button button-secondary'
                      type='button'
                      onClick={() => beginReply(post)}
                    >
                      Reply
                    </button>
                  </div>
                </article>
              </li>
            ))}
          </ul>
        </section>

        <section className='panel'>
          <div className='panel-header'>
            <h2>Thread</h2>
            {selectedThread ? (
              <button
                className='button button-secondary'
                type='button'
                onClick={() => {
                  setSelectedThread(null);
                  setThread([]);
                  clearReply();
                }}
              >
                Close
              </button>
            ) : null}
          </div>
          {selectedThread ? (
            <ul className='thread-list'>
              {thread.map((post) => (
                <li key={post.id} className='thread-item'>
                  <strong>{post.content}</strong>
                  <small>{post.author_npub}</small>
                  <div className='post-actions'>
                    <button
                      className='button button-secondary'
                      type='button'
                      onClick={() => beginReply(post)}
                    >
                      Reply
                    </button>
                  </div>
                </li>
              ))}
            </ul>
          ) : (
            <p className='empty'>Select a post to inspect the thread.</p>
          )}
        </section>
      </section>

      {error ? <p className='error'>{error}</p> : null}
    </main>
  );
}
